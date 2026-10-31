use std::{
    fs::{self, File},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail, ensure};
use laputa_domain::{
    DerivedArtifact, GenerationJob, GenerationJobId, GenerationJobState, GenerationProviderOutput,
    GenerationRequest, GenerationSource, GenerationState, MediaKind, ProjectManifest,
    ProviderProvenance,
};
use laputa_provider::{
    GenerationMediaInput, GenerationProviderError, GenerationProviderErrorKind,
    GenerationProviderHandle, GenerationProviderStatus, VideoGenerationProvider,
    VideoGenerationRequest,
};

use crate::{FfprobeMediaProbe, GenerationStore, MediaProbe, sha256_file};

const GENERATION_DERIVATIVES_DIRECTORY: &str = ".laputa/derivatives/generation";
const DEFAULT_MAX_POLLS: usize = 64;

pub trait GenerationOutputMaterializer: Send + Sync {
    fn materialize(&self, output: &GenerationProviderOutput, destination: &Path) -> Result<()>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct LocalFileGenerationOutputMaterializer;

impl GenerationOutputMaterializer for LocalFileGenerationOutputMaterializer {
    fn materialize(&self, output: &GenerationProviderOutput, destination: &Path) -> Result<()> {
        output
            .validate()
            .context("provider output is invalid before materialization")?;
        let locator = output
            .locator
            .strip_prefix("file://")
            .unwrap_or(&output.locator);
        let source = Path::new(locator);
        ensure!(
            source.is_file(),
            "generation output source is not a readable local file: {}",
            source.display()
        );
        let parent = destination
            .parent()
            .context("generation output destination has no parent directory")?;
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create generation derivative directory {}",
                parent.display()
            )
        })?;
        let temporary = parent.join(format!(
            ".{}.tmp-{}",
            destination
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("generation-output"),
            std::process::id()
        ));
        fs::copy(source, &temporary).with_context(|| {
            format!(
                "failed to materialize generation output {} to {}",
                source.display(),
                temporary.display()
            )
        })?;
        File::open(&temporary)
            .and_then(|file| file.sync_all())
            .with_context(|| {
                format!(
                    "failed to sync materialized generation output {}",
                    temporary.display()
                )
            })?;
        install_replacement(&temporary, destination)
    }
}

pub trait GenerationOutputValidator: Send + Sync {
    fn validate(&self, path: &Path, expected_kind: MediaKind) -> Result<()>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct FfprobeGenerationOutputValidator;

impl GenerationOutputValidator for FfprobeGenerationOutputValidator {
    fn validate(&self, path: &Path, expected_kind: MediaKind) -> Result<()> {
        let metadata = fs::metadata(path)
            .with_context(|| format!("failed to inspect materialized output {}", path.display()))?;
        ensure!(
            metadata.is_file(),
            "materialized generation output is not a file"
        );
        ensure!(
            metadata.len() > 0,
            "materialized generation output is empty"
        );

        let technical = FfprobeMediaProbe
            .probe(path, expected_kind)
            .context("ffprobe rejected materialized generation output")?;
        ensure!(
            technical.width.is_some_and(|width| width > 0)
                && technical.height.is_some_and(|height| height > 0),
            "materialized generation output has no valid visual dimensions"
        );
        match expected_kind {
            MediaKind::Image => Ok(()),
            MediaKind::Video => {
                ensure!(
                    technical.duration_ms.is_some_and(|duration| duration > 0),
                    "materialized video output has no positive duration"
                );
                ensure!(
                    technical.video_codec.is_some(),
                    "materialized video output has no video codec"
                );
                Ok(())
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct GenerationOrchestrator {
    project_root: PathBuf,
    store: GenerationStore,
    max_polls: usize,
}

impl GenerationOrchestrator {
    #[must_use]
    pub fn at(project_root: impl Into<PathBuf>) -> Self {
        let project_root = project_root.into();
        Self {
            store: GenerationStore::at(project_root.clone()),
            project_root,
            max_polls: DEFAULT_MAX_POLLS,
        }
    }

    pub fn with_max_polls(mut self, max_polls: usize) -> Result<Self> {
        ensure!(
            max_polls > 0,
            "generation max_polls must be greater than zero"
        );
        self.max_polls = max_polls;
        Ok(self)
    }

    pub async fn submit_video_generation<P: VideoGenerationProvider>(
        &self,
        project: &ProjectManifest,
        request: GenerationRequest,
        instruction: impl Into<String>,
        provider: &P,
    ) -> Result<GenerationJobId> {
        let mut state = self.load_state(project)?;
        let inputs = self.resolve_inputs(project, &state, &request)?;
        let provider_request = VideoGenerationRequest::new(instruction, inputs)
            .map_err(anyhow::Error::new)
            .context("invalid video generation provider request")?;
        let job = GenerationJob::new(request)
            .map_err(anyhow::Error::new)
            .context("invalid durable generation request")?;
        let job_id = job.id();
        state.jobs.push(job);
        self.store
            .save(project, &state)
            .context("failed to persist queued generation job")?;

        let handle = match provider.submit(&provider_request).await {
            Ok(handle) => handle,
            Err(error) => {
                self.fail_job(
                    project,
                    &mut state,
                    job_id,
                    None,
                    provider_error_message("generation submission failed", &error),
                )?;
                return Err(anyhow::Error::new(error).context("generation submission failed"));
            }
        };
        self.transition_job(
            project,
            &mut state,
            job_id,
            GenerationJobState::Submitted {
                provenance: handle.provenance().clone(),
            },
        )?;
        Ok(job_id)
    }

    pub async fn run_video_generation<P, M, V>(
        &self,
        project: &ProjectManifest,
        request: GenerationRequest,
        instruction: impl Into<String>,
        provider: &P,
        materializer: &M,
        validator: &V,
    ) -> Result<DerivedArtifact>
    where
        P: VideoGenerationProvider,
        M: GenerationOutputMaterializer,
        V: GenerationOutputValidator,
    {
        let job_id = self
            .submit_video_generation(project, request, instruction, provider)
            .await?;
        self.resume_video_generation(project, job_id, provider, materializer, validator)
            .await
    }

    pub async fn resume_video_generation<P, M, V>(
        &self,
        project: &ProjectManifest,
        job_id: GenerationJobId,
        provider: &P,
        materializer: &M,
        validator: &V,
    ) -> Result<DerivedArtifact>
    where
        P: VideoGenerationProvider,
        M: GenerationOutputMaterializer,
        V: GenerationOutputValidator,
    {
        let mut state = self.load_state(project)?;
        if let Some(artifact) = state
            .artifacts
            .iter()
            .find(|artifact| artifact.job_id() == job_id)
        {
            return Ok(artifact.clone());
        }

        let (handle, completed_output) = match job_state(&state, job_id)? {
            GenerationJobState::Submitted { provenance }
            | GenerationJobState::Running { provenance } => (
                Some(
                    GenerationProviderHandle::new(provenance.clone())
                        .map_err(anyhow::Error::new)
                        .context("persisted generation provider handle is invalid")?,
                ),
                None,
            ),
            GenerationJobState::Completed { provenance, output } => (
                Some(
                    GenerationProviderHandle::new(provenance.clone())
                        .map_err(anyhow::Error::new)
                        .context("persisted completed provider handle is invalid")?,
                ),
                Some(output.clone()),
            ),
            GenerationJobState::Queued => {
                bail!("queued generation job has no provider handle to resume")
            }
            GenerationJobState::Failed { message, .. } => {
                bail!("generation job already failed: {message}")
            }
            GenerationJobState::Cancelled { .. } => bail!("generation job was cancelled"),
        };

        if let Some(output) = completed_output {
            return self.finalize_video_artifact(
                project,
                &mut state,
                job_id,
                &output,
                materializer,
                validator,
            );
        }

        let handle = handle.expect("submitted/running generation job must have a provider handle");
        for _ in 0..self.max_polls {
            let status = match provider.poll(&handle).await {
                Ok(status) => status,
                Err(error) => {
                    if error.kind() == GenerationProviderErrorKind::Unavailable {
                        return Err(anyhow::Error::new(error).context(
                            "generation polling is temporarily unavailable; durable job remains resumable",
                        ));
                    }
                    self.fail_job(
                        project,
                        &mut state,
                        job_id,
                        Some(handle.provenance().clone()),
                        provider_error_message("generation polling failed", &error),
                    )?;
                    return Err(anyhow::Error::new(error).context("generation polling failed"));
                }
            };

            match status {
                GenerationProviderStatus::Submitted => {
                    self.ensure_phase_or_fail(
                        project,
                        &mut state,
                        job_id,
                        handle.provenance(),
                        false,
                    )?;
                }
                GenerationProviderStatus::Running => {
                    self.ensure_phase_or_fail(
                        project,
                        &mut state,
                        job_id,
                        handle.provenance(),
                        true,
                    )?;
                }
                GenerationProviderStatus::Completed { output } => {
                    output
                        .validate()
                        .context("provider reported invalid completed output")?;
                    self.transition_job(
                        project,
                        &mut state,
                        job_id,
                        GenerationJobState::Completed {
                            provenance: handle.provenance().clone(),
                            output: output.clone(),
                        },
                    )?;
                    return self.finalize_video_artifact(
                        project,
                        &mut state,
                        job_id,
                        &output,
                        materializer,
                        validator,
                    );
                }
                GenerationProviderStatus::Failed { message } => {
                    self.fail_job(
                        project,
                        &mut state,
                        job_id,
                        Some(handle.provenance().clone()),
                        message.clone(),
                    )?;
                    bail!("generation provider failed: {message}")
                }
                GenerationProviderStatus::Cancelled => {
                    self.transition_job(
                        project,
                        &mut state,
                        job_id,
                        GenerationJobState::Cancelled {
                            provenance: Some(handle.provenance().clone()),
                        },
                    )?;
                    bail!("generation provider cancelled the job")
                }
            }
        }

        bail!(
            "generation poll budget exhausted after {} polls; durable job remains resumable",
            self.max_polls
        )
    }

    pub async fn cancel_video_generation<P: VideoGenerationProvider>(
        &self,
        project: &ProjectManifest,
        job_id: GenerationJobId,
        provider: &P,
    ) -> Result<GenerationProviderStatus> {
        let mut state = self.load_state(project)?;
        let current = job_state(&state, job_id)?.clone();
        let provenance = match &current {
            GenerationJobState::Queued => {
                self.transition_job(
                    project,
                    &mut state,
                    job_id,
                    GenerationJobState::Cancelled { provenance: None },
                )?;
                return Ok(GenerationProviderStatus::Cancelled);
            }
            GenerationJobState::Submitted { provenance }
            | GenerationJobState::Running { provenance } => provenance.clone(),
            GenerationJobState::Completed { output, .. } => {
                return Ok(GenerationProviderStatus::Completed {
                    output: output.clone(),
                });
            }
            GenerationJobState::Failed { message, .. } => {
                return Ok(GenerationProviderStatus::Failed {
                    message: message.clone(),
                });
            }
            GenerationJobState::Cancelled { .. } => {
                return Ok(GenerationProviderStatus::Cancelled);
            }
        };
        let handle = GenerationProviderHandle::new(provenance.clone())
            .map_err(anyhow::Error::new)
            .context("persisted generation provider handle is invalid")?;
        let status = provider
            .cancel(&handle)
            .await
            .map_err(anyhow::Error::new)
            .context("generation cancellation request failed")?;

        match &status {
            GenerationProviderStatus::Cancelled => self.transition_job(
                project,
                &mut state,
                job_id,
                GenerationJobState::Cancelled {
                    provenance: Some(provenance),
                },
            )?,
            GenerationProviderStatus::Completed { output } => {
                output
                    .validate()
                    .context("provider returned invalid completed output during cancellation")?;
                self.transition_job(
                    project,
                    &mut state,
                    job_id,
                    GenerationJobState::Completed {
                        provenance,
                        output: output.clone(),
                    },
                )?;
            }
            GenerationProviderStatus::Failed { message } => self.fail_job(
                project,
                &mut state,
                job_id,
                Some(provenance),
                message.clone(),
            )?,
            GenerationProviderStatus::Submitted | GenerationProviderStatus::Running => {
                bail!("provider did not acknowledge cancellation with a terminal status")
            }
        }
        Ok(status)
    }

    fn load_state(&self, project: &ProjectManifest) -> Result<GenerationState> {
        Ok(self.store.load(project)?.unwrap_or_default())
    }

    fn resolve_inputs(
        &self,
        project: &ProjectManifest,
        state: &GenerationState,
        request: &GenerationRequest,
    ) -> Result<Vec<GenerationMediaInput>> {
        request
            .validate()
            .map_err(anyhow::Error::new)
            .context("invalid generation source lineage")?;
        let mut inputs = Vec::with_capacity(request.sources.len());
        for source in &request.sources {
            let input = match source {
                GenerationSource::Asset {
                    asset_id,
                    source_sha256,
                } => {
                    let asset = project.asset(*asset_id).with_context(|| {
                        format!("generation source asset {asset_id} is missing")
                    })?;
                    ensure!(
                        &asset.source_sha256 == source_sha256,
                        "generation source asset {asset_id} revision is stale"
                    );
                    ensure!(
                        asset.original_path.is_file(),
                        "generation source asset file is missing: {}",
                        asset.original_path.display()
                    );
                    GenerationMediaInput::new(
                        asset.original_path.clone(),
                        asset.media_kind,
                        source_sha256.clone(),
                    )
                }
                GenerationSource::Artifact {
                    artifact_id,
                    source_sha256,
                } => {
                    let artifact = state.artifact(*artifact_id).with_context(|| {
                        format!("generation source artifact {artifact_id} is not published")
                    })?;
                    ensure!(
                        artifact.sha256() == source_sha256,
                        "generation source artifact {artifact_id} revision is stale"
                    );
                    let path = self.project_root.join(artifact.materialized_path());
                    ensure!(
                        path.is_file(),
                        "generation source artifact file is missing: {}",
                        path.display()
                    );
                    GenerationMediaInput::new(path, artifact.media_kind(), source_sha256.clone())
                }
            }
            .map_err(anyhow::Error::new)
            .context("invalid generation provider input")?;
            inputs.push(input);
        }
        Ok(inputs)
    }

    fn ensure_phase_or_fail(
        &self,
        project: &ProjectManifest,
        state: &mut GenerationState,
        job_id: GenerationJobId,
        provenance: &ProviderProvenance,
        running: bool,
    ) -> Result<()> {
        let current = job_state(state, job_id)?;
        match (current, running) {
            (GenerationJobState::Submitted { .. }, false)
            | (GenerationJobState::Running { .. }, true) => Ok(()),
            (GenerationJobState::Submitted { .. }, true) => self.transition_job(
                project,
                state,
                job_id,
                GenerationJobState::Running {
                    provenance: provenance.clone(),
                },
            ),
            (GenerationJobState::Running { .. }, false) => {
                self.fail_job(
                    project,
                    state,
                    job_id,
                    Some(provenance.clone()),
                    "generation provider status regressed from running to submitted".to_owned(),
                )?;
                bail!("generation provider status regressed from running to submitted")
            }
            _ => {
                bail!("generation provider returned non-terminal status after job became terminal")
            }
        }
    }

    fn transition_job(
        &self,
        project: &ProjectManifest,
        state: &mut GenerationState,
        job_id: GenerationJobId,
        next: GenerationJobState,
    ) -> Result<()> {
        let job = state
            .jobs
            .iter_mut()
            .find(|job| job.id() == job_id)
            .with_context(|| format!("generation job {job_id} is missing"))?;
        job.transition_to(next)
            .map_err(anyhow::Error::new)
            .context("invalid durable generation job transition")?;
        self.store
            .save(project, state)
            .context("failed to persist durable generation job transition")
    }

    fn fail_job(
        &self,
        project: &ProjectManifest,
        state: &mut GenerationState,
        job_id: GenerationJobId,
        provenance: Option<ProviderProvenance>,
        message: String,
    ) -> Result<()> {
        let message = if message.trim().is_empty() {
            "generation provider failed without a message".to_owned()
        } else {
            message
        };
        self.transition_job(
            project,
            state,
            job_id,
            GenerationJobState::Failed {
                provenance,
                message,
            },
        )
    }

    fn finalize_video_artifact<M, V>(
        &self,
        project: &ProjectManifest,
        state: &mut GenerationState,
        job_id: GenerationJobId,
        output: &GenerationProviderOutput,
        materializer: &M,
        validator: &V,
    ) -> Result<DerivedArtifact>
    where
        M: GenerationOutputMaterializer,
        V: GenerationOutputValidator,
    {
        if let Some(artifact) = state
            .artifacts
            .iter()
            .find(|artifact| artifact.job_id() == job_id)
        {
            return Ok(artifact.clone());
        }
        output
            .validate()
            .context("cannot materialize invalid provider output")?;
        let relative = PathBuf::from(GENERATION_DERIVATIVES_DIRECTORY)
            .join(job_id.to_string())
            .join("output.mp4");
        let destination = self.project_root.join(&relative);

        if let Err(error) = materializer.materialize(output, &destination) {
            remove_file_if_exists(&destination);
            return Err(error).context("failed to materialize completed generation output");
        }
        if let Err(error) = validator.validate(&destination, MediaKind::Video) {
            remove_file_if_exists(&destination);
            return Err(error).context("materialized generation output failed validation");
        }

        let metadata = fs::metadata(&destination).with_context(|| {
            format!(
                "failed to inspect verified generation output {}",
                destination.display()
            )
        })?;
        ensure!(
            metadata.is_file(),
            "verified generation output is not a file"
        );
        ensure!(metadata.len() > 0, "verified generation output is empty");
        let sha256 = sha256_file(&destination)?;
        let job = state
            .job(job_id)
            .with_context(|| format!("completed generation job {job_id} is missing"))?
            .clone();
        let artifact = DerivedArtifact::from_verified_materialization(
            &job,
            MediaKind::Video,
            relative,
            sha256,
            metadata.len(),
        )
        .map_err(anyhow::Error::new)
        .context("verified generation output could not be published as an artifact")?;
        state.artifacts.push(artifact.clone());
        self.store
            .save(project, state)
            .context("failed to persist published generation artifact")?;
        Ok(artifact)
    }
}

fn job_state(state: &GenerationState, job_id: GenerationJobId) -> Result<&GenerationJobState> {
    state
        .job(job_id)
        .map(GenerationJob::state)
        .with_context(|| format!("generation job {job_id} is missing"))
}

fn provider_error_message(prefix: &str, error: &GenerationProviderError) -> String {
    if error.message().trim().is_empty() {
        prefix.to_owned()
    } else {
        format!("{prefix}: {}", error.message())
    }
}

fn install_replacement(temporary: &Path, destination: &Path) -> Result<()> {
    match fs::rename(temporary, destination) {
        Ok(()) => Ok(()),
        Err(error) if destination.exists() => {
            fs::remove_file(destination).with_context(|| {
                format!(
                    "failed to replace materialized generation output after rename error: {error}"
                )
            })?;
            fs::rename(temporary, destination)
                .context("failed to install replacement materialized generation output")
        }
        Err(error) => Err(error).context("failed to install materialized generation output"),
    }
}

fn remove_file_if_exists(path: &Path) {
    if path.exists() {
        let _ = fs::remove_file(path);
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        future::Future,
        path::{Path, PathBuf},
        pin::pin,
        process::Command,
        task::{Context, Poll, Waker},
    };

    use anyhow::{Context as _, Result, ensure};
    use laputa_domain::{
        Asset, AssetId, AssetTechnicalMetadata, GenerationJobState, GenerationOperation,
        GenerationRequest, GenerationSource, MediaKind, ProjectManifest,
    };
    use laputa_provider::{
        FakeGenerationScenario, FakeGenerationStep, FakeVideoGenerationProvider,
        GenerationProviderStatus,
    };
    use tempfile::tempdir;

    use crate::{GenerationStore, sha256_file};

    use super::{
        FfprobeGenerationOutputValidator, GenerationOrchestrator,
        LocalFileGenerationOutputMaterializer,
    };

    fn block_on_ready<F: Future>(future: F) -> F::Output {
        let mut context = Context::from_waker(Waker::noop());
        let mut future = pin!(future);
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => output,
            Poll::Pending => panic!("fake generation orchestration unexpectedly yielded pending"),
        }
    }

    fn project_with_original(root: &Path) -> Result<(ProjectManifest, PathBuf)> {
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

    fn request_for(project: &ProjectManifest) -> GenerationRequest {
        let asset = &project.assets[0];
        GenerationRequest::new(
            GenerationOperation::AnimatePhoto,
            vec![GenerationSource::Asset {
                asset_id: asset.id,
                source_sha256: asset.source_sha256.clone(),
            }],
        )
    }

    fn write_video_fixture(path: &Path) -> Result<()> {
        let output = Command::new("ffmpeg")
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-f",
                "lavfi",
                "-i",
                "color=c=black:s=64x64:d=0.25:r=10",
                "-an",
                "-c:v",
                "libx264",
                "-pix_fmt",
                "yuv420p",
                "-y",
            ])
            .arg(path)
            .output()
            .context("failed to start ffmpeg fixture")?;
        ensure!(
            output.status.success(),
            "ffmpeg fixture failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        Ok(())
    }

    #[test]
    fn completed_video_is_materialized_verified_and_published() -> Result<()> {
        let temp = tempdir()?;
        let project_root = temp.path().join("project");
        let (project, original) = project_with_original(temp.path())?;
        let original_hash = sha256_file(&original)?;
        let provider_output = temp.path().join("provider-output.mp4");
        write_video_fixture(&provider_output)?;
        let provider = FakeVideoGenerationProvider::new(FakeGenerationScenario::Scripted(vec![
            FakeGenerationStep::Running,
            FakeGenerationStep::Completed {
                locator: provider_output.display().to_string(),
            },
        ]));
        let orchestrator = GenerationOrchestrator::at(&project_root);

        let artifact = block_on_ready(orchestrator.run_video_generation(
            &project,
            request_for(&project),
            "Create gentle motion from this photo",
            &provider,
            &LocalFileGenerationOutputMaterializer,
            &FfprobeGenerationOutputValidator,
        ))?;

        let materialized = project_root.join(artifact.materialized_path());
        assert!(materialized.is_file());
        assert_eq!(artifact.sha256(), sha256_file(&materialized)?);
        assert_eq!(sha256_file(&original)?, original_hash);
        let state = GenerationStore::at(&project_root)
            .load(&project)?
            .expect("generation state should exist");
        assert_eq!(state.artifacts.len(), 1);
        assert!(matches!(
            state.job(artifact.job_id()).expect("job exists").state(),
            GenerationJobState::Completed { .. }
        ));
        Ok(())
    }

    #[test]
    fn provider_failure_is_persisted_without_artifact_or_original_mutation() -> Result<()> {
        let temp = tempdir()?;
        let project_root = temp.path().join("project");
        let (project, original) = project_with_original(temp.path())?;
        let original_hash = sha256_file(&original)?;
        let provider = FakeVideoGenerationProvider::new(FakeGenerationScenario::Scripted(vec![
            FakeGenerationStep::Running,
            FakeGenerationStep::Failed {
                message: "fixture failure".to_owned(),
            },
        ]));
        let orchestrator = GenerationOrchestrator::at(&project_root);

        let result = block_on_ready(orchestrator.run_video_generation(
            &project,
            request_for(&project),
            "Create motion",
            &provider,
            &LocalFileGenerationOutputMaterializer,
            &FfprobeGenerationOutputValidator,
        ));
        assert!(result.is_err());
        assert_eq!(sha256_file(&original)?, original_hash);
        let state = GenerationStore::at(&project_root)
            .load(&project)?
            .expect("generation state should exist");
        assert!(state.artifacts.is_empty());
        assert!(matches!(
            state.jobs[0].state(),
            GenerationJobState::Failed { .. }
        ));
        Ok(())
    }

    #[test]
    fn submitted_job_can_be_cancelled_and_reopened_without_artifact() -> Result<()> {
        let temp = tempdir()?;
        let project_root = temp.path().join("project");
        let (project, original) = project_with_original(temp.path())?;
        let original_hash = sha256_file(&original)?;
        let provider = FakeVideoGenerationProvider::new(FakeGenerationScenario::Scripted(vec![
            FakeGenerationStep::Running,
            FakeGenerationStep::Completed {
                locator: "must-not-materialize.mp4".to_owned(),
            },
        ]));
        let orchestrator = GenerationOrchestrator::at(&project_root);
        let job_id = block_on_ready(orchestrator.submit_video_generation(
            &project,
            request_for(&project),
            "Create motion",
            &provider,
        ))?;

        let status =
            block_on_ready(orchestrator.cancel_video_generation(&project, job_id, &provider))?;
        assert_eq!(status, GenerationProviderStatus::Cancelled);
        assert_eq!(sha256_file(&original)?, original_hash);
        let state = GenerationStore::at(&project_root)
            .load(&project)?
            .expect("generation state should exist");
        assert!(state.artifacts.is_empty());
        assert!(matches!(
            state.job(job_id).expect("job exists").state(),
            GenerationJobState::Cancelled { .. }
        ));
        Ok(())
    }

    #[test]
    fn malformed_completion_becomes_failed_job_without_artifact() -> Result<()> {
        let temp = tempdir()?;
        let project_root = temp.path().join("project");
        let (project, original) = project_with_original(temp.path())?;
        let original_hash = sha256_file(&original)?;
        let provider = FakeVideoGenerationProvider::new(FakeGenerationScenario::Scripted(vec![
            FakeGenerationStep::MalformedCompletion,
        ]));
        let orchestrator = GenerationOrchestrator::at(&project_root);

        let result = block_on_ready(orchestrator.run_video_generation(
            &project,
            request_for(&project),
            "Create motion",
            &provider,
            &LocalFileGenerationOutputMaterializer,
            &FfprobeGenerationOutputValidator,
        ));
        assert!(result.is_err());
        assert_eq!(sha256_file(&original)?, original_hash);
        let state = GenerationStore::at(&project_root)
            .load(&project)?
            .expect("generation state should exist");
        assert!(state.artifacts.is_empty());
        assert!(matches!(
            state.jobs[0].state(),
            GenerationJobState::Failed { .. }
        ));
        Ok(())
    }

    #[test]
    fn completed_but_missing_output_stays_completed_without_published_artifact() -> Result<()> {
        let temp = tempdir()?;
        let project_root = temp.path().join("project");
        let (project, original) = project_with_original(temp.path())?;
        let original_hash = sha256_file(&original)?;
        let missing = temp.path().join("missing-provider-output.mp4");
        let provider = FakeVideoGenerationProvider::new(FakeGenerationScenario::Scripted(vec![
            FakeGenerationStep::Completed {
                locator: missing.display().to_string(),
            },
        ]));
        let orchestrator = GenerationOrchestrator::at(&project_root);

        let result = block_on_ready(orchestrator.run_video_generation(
            &project,
            request_for(&project),
            "Create motion",
            &provider,
            &LocalFileGenerationOutputMaterializer,
            &FfprobeGenerationOutputValidator,
        ));
        assert!(result.is_err());
        assert_eq!(sha256_file(&original)?, original_hash);
        let state = GenerationStore::at(&project_root)
            .load(&project)?
            .expect("generation state should exist");
        assert!(state.artifacts.is_empty());
        assert!(matches!(
            state.jobs[0].state(),
            GenerationJobState::Completed { .. }
        ));
        Ok(())
    }
}
