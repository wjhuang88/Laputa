use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
};

use anyhow::{Context, Result, ensure};
use laputa_domain::{GenerationState, ProjectManifest};
use tempfile::NamedTempFile;

const STATE_DIRECTORY: &str = ".laputa";
const GENERATION_STATE_FILE: &str = "generation.json";
const GENERATION_LOCK_FILE: &str = "generation.json.lock";

#[derive(Clone, Debug)]
pub struct GenerationStore {
    project_root: PathBuf,
}

impl GenerationStore {
    #[must_use]
    pub fn at(project_root: impl Into<PathBuf>) -> Self {
        Self {
            project_root: project_root.into(),
        }
    }

    #[must_use]
    pub fn path(&self) -> PathBuf {
        self.project_root
            .join(STATE_DIRECTORY)
            .join(GENERATION_STATE_FILE)
    }

    pub fn load(&self, project: &ProjectManifest) -> Result<Option<GenerationState>> {
        let path = self.path();
        if !path.exists() {
            return Ok(None);
        }
        let bytes = fs::read(&path)
            .with_context(|| format!("failed to read generation state {}", path.display()))?;
        let state: GenerationState = serde_json::from_slice(&bytes)
            .with_context(|| format!("invalid generation state {}", path.display()))?;
        state
            .validate_for(project)
            .context("persisted generation state is invalid for the current project")?;
        Ok(Some(state))
    }

    /// Serialize a read/modify/write of the *latest* state across independent processes.
    ///
    /// The sibling lock file is stable and must never be replaced or deleted.
    /// Do not call provider/network operations while holding this synchronous lock.
    pub fn update<T>(
        &self,
        project: &ProjectManifest,
        mutate: impl FnOnce(&mut GenerationState) -> Result<T>,
    ) -> Result<T> {
        let path = self.path();
        let parent = path
            .parent()
            .context("generation state path has no parent directory")?;
        fs::create_dir_all(parent).with_context(|| {
            format!("failed to create generation state directory {}", parent.display())
        })?;
        let lock_path = parent.join(GENERATION_LOCK_FILE);
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)
            .with_context(|| format!("failed to open generation lock {}", lock_path.display()))?;
        lock.lock()
            .with_context(|| format!("failed to lock generation state {}", lock_path.display()))?;

        let mut state = self.load(project)?.unwrap_or_default();
        let result = mutate(&mut state)?;
        self.persist_state(project, &state)?;
        // Dropping the handle releases the lock on success and on every failure path.
        Ok(result)
    }

    /// Replace the entire state, rejecting stale snapshots that remove or roll back
    /// committed jobs/artifacts. Callers doing read/modify/write should use `update`.
    pub fn save(&self, project: &ProjectManifest, state: &GenerationState) -> Result<()> {
        self.update(project, |current| {
            for previous in &current.jobs {
                let next = state.job(previous.id()).with_context(|| {
                    format!("snapshot would discard durable generation job {}", previous.id())
                })?;
                ensure!(
                    next.request() == previous.request(),
                    "snapshot would rewrite a committed generation request"
                );
                if next.state() != previous.state() {
                    let mut checked = previous.clone();
                    checked
                        .transition_to(next.state().clone())
                        .map_err(anyhow::Error::new)
                        .context("snapshot would roll back or skip a generation transition")?;
                }
            }
            for artifact in &current.artifacts {
                ensure!(
                    state.artifacts.contains(artifact),
                    "snapshot would remove or rewrite a published generation artifact"
                );
            }
            *current = state.clone();
            Ok(())
        })
    }

    fn persist_state(&self, project: &ProjectManifest, state: &GenerationState) -> Result<()> {
        state
            .validate_for(project)
            .context("cannot persist invalid generation state")?;
        let path = self.path();
        let parent = path
            .parent()
            .context("generation state path has no parent directory")?;
        let bytes =
            serde_json::to_vec_pretty(state).context("failed to serialize generation state")?;
        // Each writer receives an exclusive, distinct tempfile in the target directory.
        let mut temporary = NamedTempFile::new_in(parent).with_context(|| {
            format!("failed to create temporary generation state in {}", parent.display())
        })?;
        temporary
            .write_all(&bytes)
            .context("failed to write temporary generation state")?;
        temporary
            .as_file()
            .sync_all()
            .context("failed to sync temporary generation state")?;
        // Never remove a valid destination when replacement fails.
        let installed = temporary.persist(&path).map_err(|error| {
            anyhow::Error::new(error.error).context(format!(
                "failed to install generation state {}", path.display()
            ))
        })?;
        drop(installed);
        #[cfg(unix)]
        std::fs::File::open(parent)
            .and_then(|directory| directory.sync_all())
            .with_context(|| {
                format!("failed to sync generation state directory {}", parent.display())
            })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use anyhow::Result;
    use laputa_domain::{
        Asset, AssetId, AssetTechnicalMetadata, DerivedArtifact, GenerationJob, GenerationJobState,
        GenerationOperation, GenerationProviderOutput, GenerationRequest, GenerationSource,
        GenerationState, MediaKind, ProjectManifest, ProviderProvenance,
    };
    use tempfile::tempdir;

    use crate::sha256_file;

    use super::GenerationStore;

    fn digest(character: char) -> String {
        std::iter::repeat_n(character, 64).collect()
    }

    fn project_with_original(root: &std::path::Path) -> Result<(ProjectManifest, PathBuf)> {
        let original = root.join("photo.jpg");
        fs::write(&original, b"original-photo-bytes")?;
        let source_sha256 = sha256_file(&original)?;
        let mut project = ProjectManifest::new("Trip");
        project.assets.push(Asset {
            id: AssetId::new(),
            original_path: original.clone(),
            display_name: "photo.jpg".to_owned(),
            media_kind: MediaKind::Image,
            source_sha256,
            byte_len: fs::metadata(&original)?.len(),
            technical: AssetTechnicalMetadata::default(),
        });
        Ok((project, original))
    }

    fn provenance() -> ProviderProvenance {
        ProviderProvenance {
            provider: "fixture-provider".to_owned(),
            request_id: Some("request-1".to_owned()),
            job_id: Some("job-1".to_owned()),
        }
    }

    #[test]
    fn generation_state_reopens_with_non_terminal_job() -> Result<()> {
        let temp = tempdir()?;
        let (project, original) = project_with_original(temp.path())?;
        let original_hash = sha256_file(&original)?;
        let asset = &project.assets[0];
        let request = GenerationRequest::new(
            GenerationOperation::AnimatePhoto,
            vec![GenerationSource::Asset {
                asset_id: asset.id,
                source_sha256: asset.source_sha256.clone(),
            }],
        );
        let mut job = GenerationJob::new(request)?;
        job.transition_to(GenerationJobState::Submitted {
            provenance: provenance(),
        })?;
        job.transition_to(GenerationJobState::Running {
            provenance: provenance(),
        })?;
        let mut state = GenerationState::new();
        state.jobs.push(job);

        let store = GenerationStore::at(temp.path().join("project"));
        store.save(&project, &state)?;
        let reopened = store
            .load(&project)?
            .expect("generation state should exist");

        assert_eq!(reopened, state);
        assert_eq!(sha256_file(&original)?, original_hash);
        Ok(())
    }

    #[test]
    fn completed_artifact_lineage_reopens() -> Result<()> {
        let temp = tempdir()?;
        let (project, original) = project_with_original(temp.path())?;
        let original_hash = sha256_file(&original)?;
        let asset = &project.assets[0];
        let request = GenerationRequest::new(
            GenerationOperation::AnimatePhoto,
            vec![GenerationSource::Asset {
                asset_id: asset.id,
                source_sha256: asset.source_sha256.clone(),
            }],
        );
        let mut job = GenerationJob::new(request)?;
        job.transition_to(GenerationJobState::Submitted {
            provenance: provenance(),
        })?;
        job.transition_to(GenerationJobState::Completed {
            provenance: provenance(),
            output: GenerationProviderOutput {
                locator: "fixture://output".to_owned(),
            },
        })?;
        let artifact = DerivedArtifact::from_verified_materialization(
            &job,
            MediaKind::Video,
            PathBuf::from(".laputa/derivatives/generated.mp4"),
            digest('b'),
            100,
        )?;
        let mut state = GenerationState::new();
        state.jobs.push(job);
        state.artifacts.push(artifact);

        let store = GenerationStore::at(temp.path().join("project"));
        store.save(&project, &state)?;
        let reopened = store
            .load(&project)?
            .expect("generation state should exist");

        assert_eq!(reopened, state);
        assert_eq!(sha256_file(&original)?, original_hash);
        Ok(())
    }

    #[test]
    fn stale_source_revision_is_not_persisted() -> Result<()> {
        let temp = tempdir()?;
        let (project, _) = project_with_original(temp.path())?;
        let asset = &project.assets[0];
        let request = GenerationRequest::new(
            GenerationOperation::AnimatePhoto,
            vec![GenerationSource::Asset {
                asset_id: asset.id,
                source_sha256: digest('f'),
            }],
        );
        let mut state = GenerationState::new();
        state.jobs.push(GenerationJob::new(request)?);
        let store = GenerationStore::at(temp.path().join("project"));

        assert!(store.save(&project, &state).is_err());
        assert!(!store.path().exists());
        Ok(())
    }

    #[test]
    fn failed_transaction_preserves_last_committed_state() -> Result<()> {
        let temp = tempdir()?;
        let (project, original) = project_with_original(temp.path())?;
        let original_hash = sha256_file(&original)?;
        let store = GenerationStore::at(temp.path().join("project"));
        let job = GenerationJob::new(GenerationRequest::new(
            GenerationOperation::GenerateBroll, vec![],
        ))?;
        store.update(&project, |state| {
            state.jobs.push(job.clone());
            Ok(())
        })?;
        let bytes = fs::read(store.path())?;
        let aborted = store.update(&project, |state| -> Result<()> {
            state.jobs.clear();
            anyhow::bail!("injected transaction failure")
        });
        assert!(aborted.is_err());
        assert_eq!(fs::read(store.path())?, bytes);
        let invalid = store.update(&project, |state| {
            state.jobs.push(job.clone());
            Ok(())
        });
        assert!(invalid.is_err());
        assert_eq!(fs::read(store.path())?, bytes);
        assert_eq!(sha256_file(&original)?, original_hash);
        Ok(())
    }

    #[test]
    fn concurrent_independent_stores_preserve_every_job() -> Result<()> {
        let temp = tempdir()?;
        let root = temp.path().join("project");
        let project = ProjectManifest::new("Concurrent");
        let writers: Vec<_> = (0..4)
            .map(|_| {
                let root = root.clone();
                let project = project.clone();
                std::thread::spawn(move || -> Result<()> {
                    for _ in 0..12 {
                        GenerationStore::at(&root).update(&project, |state| {
                            state.jobs.push(GenerationJob::new(GenerationRequest::new(
                                GenerationOperation::GenerateBroll, vec![],
                            ))?);
                            std::thread::yield_now();
                            Ok(())
                        })?;
                    }
                    Ok(())
                })
            })
            .collect();
        for writer in writers {
            writer.join().expect("writer panicked")?;
        }
        let state = GenerationStore::at(&root)
            .load(&project)?
            .expect("state should exist");
        assert_eq!(state.jobs.len(), 48);
        Ok(())
    }

    #[test]
    fn cross_process_writer_worker() -> Result<()> {
        let Ok(root) = std::env::var("LAPUTA_GENERATION_TEST_ROOT") else {
            return Ok(());
        };
        let project = ProjectManifest::new("Concurrent");
        for _ in 0..12 {
            GenerationStore::at(&root).update(&project, |state| {
                state.jobs.push(GenerationJob::new(GenerationRequest::new(
                    GenerationOperation::GenerateBroll, vec![],
                ))?);
                Ok(())
            })?;
        }
        Ok(())
    }

    #[test]
    fn cross_process_writers_preserve_every_job() -> Result<()> {
        let temp = tempdir()?;
        let root = temp.path().join("project");
        let test_binary = std::env::current_exe()?;
        let mut workers = Vec::new();
        for _ in 0..3 {
            workers.push(std::process::Command::new(&test_binary)
                .arg("--exact")
                .arg("generation_store::tests::cross_process_writer_worker")
                .env("LAPUTA_GENERATION_TEST_ROOT", &root)
                .spawn()?);
        }
        for mut worker in workers {
            assert!(worker.wait()?.success(), "cross-process writer failed");
        }
        let project = ProjectManifest::new("Concurrent");
        let state = GenerationStore::at(&root)
            .load(&project)?
            .expect("state should exist");
        assert_eq!(state.jobs.len(), 36);
        Ok(())
    }

    #[test]
    fn stale_whole_snapshot_cannot_erase_a_newly_submitted_job() -> Result<()> {
        let temp = tempdir()?;
        let project = ProjectManifest::new("Concurrent");
        let store = GenerationStore::at(temp.path().join("project"));
        let first = GenerationJob::new(GenerationRequest::new(
            GenerationOperation::GenerateBroll, vec![],
        ))?;
        store.update(&project, |state| {
            state.jobs.push(first.clone());
            Ok(())
        })?;
        let stale = store.load(&project)?.expect("first snapshot");
        let second = GenerationJob::new(GenerationRequest::new(
            GenerationOperation::GenerateBroll, vec![],
        ))?;
        store.update(&project, |state| {
            state.jobs.push(second.clone());
            Ok(())
        })?;
        let bytes = fs::read(store.path())?;
        assert!(store.save(&project, &stale).is_err());
        assert_eq!(fs::read(store.path())?, bytes);
        let reopened = store.load(&project)?.expect("both jobs remain durable");
        assert!(reopened.job(first.id()).is_some());
        assert!(reopened.job(second.id()).is_some());
        Ok(())
    }
}
