use std::{
    fs::{self, File},
    path::PathBuf,
};

use anyhow::{Context, Result};
use laputa_domain::{GenerationState, ProjectManifest};

const STATE_DIRECTORY: &str = ".laputa";
const GENERATION_STATE_FILE: &str = "generation.json";

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

    pub fn save(&self, project: &ProjectManifest, state: &GenerationState) -> Result<()> {
        state
            .validate_for(project)
            .context("cannot persist invalid generation state")?;
        let path = self.path();
        let parent = path
            .parent()
            .context("generation state path has no parent directory")?;
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create generation state directory {}",
                parent.display()
            )
        })?;
        let bytes =
            serde_json::to_vec_pretty(state).context("failed to serialize generation state")?;
        let temporary = parent.join(format!(
            ".{GENERATION_STATE_FILE}.tmp-{}",
            std::process::id()
        ));
        fs::write(&temporary, bytes).with_context(|| {
            format!(
                "failed to write temporary generation state {}",
                temporary.display()
            )
        })?;
        File::open(&temporary)
            .and_then(|file| file.sync_all())
            .with_context(|| {
                format!(
                    "failed to sync temporary generation state {}",
                    temporary.display()
                )
            })?;
        install_replacement(&temporary, &path)
    }
}

fn install_replacement(temporary: &std::path::Path, destination: &std::path::Path) -> Result<()> {
    match fs::rename(temporary, destination) {
        Ok(()) => Ok(()),
        Err(error) if destination.exists() => {
            fs::remove_file(destination).with_context(|| {
                format!("failed to replace generation state after rename error: {error}")
            })?;
            fs::rename(temporary, destination)
                .context("failed to install replacement generation state")
        }
        Err(error) => Err(error).context("failed to install generation state"),
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use anyhow::Result;
    use laputa_domain::{
        Asset, AssetId, AssetTechnicalMetadata, DerivedArtifact, GenerationJob,
        GenerationJobState, GenerationOperation, GenerationProviderOutput, GenerationRequest,
        GenerationSource, GenerationState, MediaKind, ProjectManifest, ProviderProvenance,
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
        let reopened = store.load(&project)?.expect("generation state should exist");

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
        let reopened = store.load(&project)?.expect("generation state should exist");

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
}
