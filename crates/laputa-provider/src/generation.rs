use std::{
    collections::{HashMap, VecDeque},
    fmt,
    path::{Path, PathBuf},
    sync::Mutex,
};

use async_trait::async_trait;
use laputa_domain::{GenerationProviderOutput, MediaKind, ProviderProvenance};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GenerationProviderErrorKind {
    Unavailable,
    Rejected,
    MalformedResponse,
    UnknownJob,
    CancellationUnsupported,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GenerationProviderError {
    kind: GenerationProviderErrorKind,
    message: String,
}

impl GenerationProviderError {
    #[must_use]
    pub fn new(kind: GenerationProviderErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    #[must_use]
    pub fn kind(&self) -> GenerationProviderErrorKind {
        self.kind
    }

    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for GenerationProviderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for GenerationProviderError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GenerationMediaInput {
    path: PathBuf,
    media_kind: MediaKind,
    source_sha256: String,
}

impl GenerationMediaInput {
    pub fn new(
        path: impl Into<PathBuf>,
        media_kind: MediaKind,
        source_sha256: impl Into<String>,
    ) -> Result<Self, GenerationProviderError> {
        let input = Self {
            path: path.into(),
            media_kind,
            source_sha256: source_sha256.into(),
        };
        input.validate()?;
        Ok(input)
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    #[must_use]
    pub fn media_kind(&self) -> MediaKind {
        self.media_kind
    }

    #[must_use]
    pub fn source_sha256(&self) -> &str {
        &self.source_sha256
    }

    fn validate(&self) -> Result<(), GenerationProviderError> {
        if self.path.as_os_str().is_empty() {
            return Err(GenerationProviderError::new(
                GenerationProviderErrorKind::Rejected,
                "generation media input path must not be empty",
            ));
        }
        validate_sha256("generation media input sha256", &self.source_sha256)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VideoGenerationRequest {
    instruction: String,
    inputs: Vec<GenerationMediaInput>,
}

impl VideoGenerationRequest {
    pub fn new(
        instruction: impl Into<String>,
        inputs: Vec<GenerationMediaInput>,
    ) -> Result<Self, GenerationProviderError> {
        let request = Self {
            instruction: instruction.into(),
            inputs,
        };
        request.validate()?;
        Ok(request)
    }

    #[must_use]
    pub fn instruction(&self) -> &str {
        &self.instruction
    }

    #[must_use]
    pub fn inputs(&self) -> &[GenerationMediaInput] {
        &self.inputs
    }

    fn validate(&self) -> Result<(), GenerationProviderError> {
        validate_instruction(&self.instruction)?;
        for input in &self.inputs {
            input.validate()?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImageGenerationRequest {
    instruction: String,
    inputs: Vec<GenerationMediaInput>,
}

impl ImageGenerationRequest {
    pub fn new(
        instruction: impl Into<String>,
        inputs: Vec<GenerationMediaInput>,
    ) -> Result<Self, GenerationProviderError> {
        let request = Self {
            instruction: instruction.into(),
            inputs,
        };
        request.validate()?;
        Ok(request)
    }

    #[must_use]
    pub fn instruction(&self) -> &str {
        &self.instruction
    }

    #[must_use]
    pub fn inputs(&self) -> &[GenerationMediaInput] {
        &self.inputs
    }

    fn validate(&self) -> Result<(), GenerationProviderError> {
        validate_instruction(&self.instruction)?;
        for input in &self.inputs {
            input.validate()?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GenerationProviderHandle {
    provenance: ProviderProvenance,
}

impl GenerationProviderHandle {
    pub fn new(provenance: ProviderProvenance) -> Result<Self, GenerationProviderError> {
        provenance.validate().map_err(|error| {
            GenerationProviderError::new(
                GenerationProviderErrorKind::MalformedResponse,
                error.to_string(),
            )
        })?;
        if provenance
            .job_id
            .as_deref()
            .is_none_or(|value| value.is_empty())
        {
            return Err(GenerationProviderError::new(
                GenerationProviderErrorKind::MalformedResponse,
                "generation provider submission must include a non-empty job id",
            ));
        }
        Ok(Self { provenance })
    }

    #[must_use]
    pub fn provenance(&self) -> &ProviderProvenance {
        &self.provenance
    }

    fn job_id(&self) -> &str {
        self.provenance
            .job_id
            .as_deref()
            .expect("validated provider handle must contain a job id")
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GenerationProviderStatus {
    Submitted,
    Running,
    Completed { output: GenerationProviderOutput },
    Failed { message: String },
    Cancelled,
}

impl GenerationProviderStatus {
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Completed { .. } | Self::Failed { .. } | Self::Cancelled
        )
    }

    fn validate(&self) -> Result<(), GenerationProviderError> {
        match self {
            Self::Submitted | Self::Running | Self::Cancelled => Ok(()),
            Self::Completed { output } => output.validate().map_err(|error| {
                GenerationProviderError::new(
                    GenerationProviderErrorKind::MalformedResponse,
                    error.to_string(),
                )
            }),
            Self::Failed { message } => {
                if message.trim().is_empty() {
                    return Err(GenerationProviderError::new(
                        GenerationProviderErrorKind::MalformedResponse,
                        "generation provider failure message must not be empty",
                    ));
                }
                Ok(())
            }
        }
    }
}

#[async_trait]
pub trait VideoGenerationProvider: Send + Sync {
    async fn submit(
        &self,
        request: &VideoGenerationRequest,
    ) -> Result<GenerationProviderHandle, GenerationProviderError>;

    async fn poll(
        &self,
        handle: &GenerationProviderHandle,
    ) -> Result<GenerationProviderStatus, GenerationProviderError>;

    async fn cancel(
        &self,
        handle: &GenerationProviderHandle,
    ) -> Result<GenerationProviderStatus, GenerationProviderError>;
}

#[async_trait]
pub trait ImageGenerationProvider: Send + Sync {
    async fn submit(
        &self,
        request: &ImageGenerationRequest,
    ) -> Result<GenerationProviderHandle, GenerationProviderError>;

    async fn poll(
        &self,
        handle: &GenerationProviderHandle,
    ) -> Result<GenerationProviderStatus, GenerationProviderError>;

    async fn cancel(
        &self,
        handle: &GenerationProviderHandle,
    ) -> Result<GenerationProviderStatus, GenerationProviderError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FakeGenerationStep {
    Submitted,
    Running,
    Completed { locator: String },
    Failed { message: String },
    Cancelled,
    MalformedCompletion,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FakeGenerationScenario {
    Scripted(Vec<FakeGenerationStep>),
    SubmissionError {
        kind: GenerationProviderErrorKind,
        message: String,
    },
}

#[derive(Debug)]
struct FakeGenerationJob {
    remaining: VecDeque<FakeGenerationStep>,
    current: GenerationProviderStatus,
}

#[derive(Debug, Default)]
struct FakeGenerationState {
    next_id: u64,
    jobs: HashMap<String, FakeGenerationJob>,
}

#[derive(Debug)]
struct FakeGenerationBackend {
    provider_name: &'static str,
    scenario: FakeGenerationScenario,
    state: Mutex<FakeGenerationState>,
}

impl FakeGenerationBackend {
    fn new(provider_name: &'static str, scenario: FakeGenerationScenario) -> Self {
        Self {
            provider_name,
            scenario,
            state: Mutex::new(FakeGenerationState::default()),
        }
    }

    fn submit(&self) -> Result<GenerationProviderHandle, GenerationProviderError> {
        if let FakeGenerationScenario::SubmissionError { kind, message } = &self.scenario {
            return Err(GenerationProviderError::new(*kind, message.clone()));
        }

        let steps = match &self.scenario {
            FakeGenerationScenario::Scripted(steps) => steps.clone(),
            FakeGenerationScenario::SubmissionError { .. } => unreachable!(),
        };
        let mut state = self.lock_state()?;
        state.next_id = state.next_id.saturating_add(1);
        let id = state.next_id;
        let job_id = format!("fake-job-{id}");
        let handle = GenerationProviderHandle::new(ProviderProvenance {
            provider: self.provider_name.to_owned(),
            request_id: Some(format!("fake-request-{id}")),
            job_id: Some(job_id.clone()),
        })?;
        state.jobs.insert(
            job_id,
            FakeGenerationJob {
                remaining: VecDeque::from(steps),
                current: GenerationProviderStatus::Submitted,
            },
        );
        Ok(handle)
    }

    fn poll(
        &self,
        handle: &GenerationProviderHandle,
    ) -> Result<GenerationProviderStatus, GenerationProviderError> {
        let mut state = self.lock_state()?;
        let job = state.jobs.get_mut(handle.job_id()).ok_or_else(|| {
            GenerationProviderError::new(
                GenerationProviderErrorKind::UnknownJob,
                format!("unknown fake generation job {}", handle.job_id()),
            )
        })?;

        if job.current.is_terminal() {
            return Ok(job.current.clone());
        }
        let Some(step) = job.remaining.pop_front() else {
            return Ok(job.current.clone());
        };
        let next = fake_step_status(step)?;
        next.validate()?;
        job.current = next.clone();
        Ok(next)
    }

    fn cancel(
        &self,
        handle: &GenerationProviderHandle,
    ) -> Result<GenerationProviderStatus, GenerationProviderError> {
        let mut state = self.lock_state()?;
        let job = state.jobs.get_mut(handle.job_id()).ok_or_else(|| {
            GenerationProviderError::new(
                GenerationProviderErrorKind::UnknownJob,
                format!("unknown fake generation job {}", handle.job_id()),
            )
        })?;
        if job.current.is_terminal() {
            return Ok(job.current.clone());
        }
        job.remaining.clear();
        job.current = GenerationProviderStatus::Cancelled;
        Ok(GenerationProviderStatus::Cancelled)
    }

    fn lock_state(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, FakeGenerationState>, GenerationProviderError> {
        self.state.lock().map_err(|_| {
            GenerationProviderError::new(
                GenerationProviderErrorKind::Unavailable,
                "fake generation provider state lock was poisoned",
            )
        })
    }
}

fn fake_step_status(
    step: FakeGenerationStep,
) -> Result<GenerationProviderStatus, GenerationProviderError> {
    match step {
        FakeGenerationStep::Submitted => Ok(GenerationProviderStatus::Submitted),
        FakeGenerationStep::Running => Ok(GenerationProviderStatus::Running),
        FakeGenerationStep::Completed { locator } => {
            let status = GenerationProviderStatus::Completed {
                output: GenerationProviderOutput { locator },
            };
            status.validate()?;
            Ok(status)
        }
        FakeGenerationStep::Failed { message } => {
            let status = GenerationProviderStatus::Failed { message };
            status.validate()?;
            Ok(status)
        }
        FakeGenerationStep::Cancelled => Ok(GenerationProviderStatus::Cancelled),
        FakeGenerationStep::MalformedCompletion => Err(GenerationProviderError::new(
            GenerationProviderErrorKind::MalformedResponse,
            "fake provider reported completion without a valid output",
        )),
    }
}

#[derive(Debug)]
pub struct FakeVideoGenerationProvider {
    backend: FakeGenerationBackend,
}

impl FakeVideoGenerationProvider {
    #[must_use]
    pub fn new(scenario: FakeGenerationScenario) -> Self {
        Self {
            backend: FakeGenerationBackend::new("fake-video-generation", scenario),
        }
    }
}

#[async_trait]
impl VideoGenerationProvider for FakeVideoGenerationProvider {
    async fn submit(
        &self,
        request: &VideoGenerationRequest,
    ) -> Result<GenerationProviderHandle, GenerationProviderError> {
        request.validate()?;
        self.backend.submit()
    }

    async fn poll(
        &self,
        handle: &GenerationProviderHandle,
    ) -> Result<GenerationProviderStatus, GenerationProviderError> {
        self.backend.poll(handle)
    }

    async fn cancel(
        &self,
        handle: &GenerationProviderHandle,
    ) -> Result<GenerationProviderStatus, GenerationProviderError> {
        self.backend.cancel(handle)
    }
}

#[derive(Debug)]
pub struct FakeImageGenerationProvider {
    backend: FakeGenerationBackend,
}

impl FakeImageGenerationProvider {
    #[must_use]
    pub fn new(scenario: FakeGenerationScenario) -> Self {
        Self {
            backend: FakeGenerationBackend::new("fake-image-generation", scenario),
        }
    }
}

#[async_trait]
impl ImageGenerationProvider for FakeImageGenerationProvider {
    async fn submit(
        &self,
        request: &ImageGenerationRequest,
    ) -> Result<GenerationProviderHandle, GenerationProviderError> {
        request.validate()?;
        self.backend.submit()
    }

    async fn poll(
        &self,
        handle: &GenerationProviderHandle,
    ) -> Result<GenerationProviderStatus, GenerationProviderError> {
        self.backend.poll(handle)
    }

    async fn cancel(
        &self,
        handle: &GenerationProviderHandle,
    ) -> Result<GenerationProviderStatus, GenerationProviderError> {
        self.backend.cancel(handle)
    }
}

fn validate_instruction(instruction: &str) -> Result<(), GenerationProviderError> {
    if instruction.trim().is_empty() {
        return Err(GenerationProviderError::new(
            GenerationProviderErrorKind::Rejected,
            "generation instruction must not be empty",
        ));
    }
    Ok(())
}

fn validate_sha256(field: &str, value: &str) -> Result<(), GenerationProviderError> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(GenerationProviderError::new(
            GenerationProviderErrorKind::Rejected,
            format!("{field} must be a 64-character hexadecimal digest"),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{
        FakeGenerationScenario, FakeGenerationStep, FakeImageGenerationProvider,
        FakeVideoGenerationProvider, GenerationMediaInput, GenerationProviderErrorKind,
        GenerationProviderStatus, ImageGenerationProvider, ImageGenerationRequest,
        VideoGenerationProvider, VideoGenerationRequest,
    };
    use laputa_domain::MediaKind;

    fn digest(character: char) -> String {
        std::iter::repeat_n(character, 64).collect()
    }

    fn video_request() -> VideoGenerationRequest {
        VideoGenerationRequest::new(
            "Create gentle motion from this photo",
            vec![GenerationMediaInput::new(
                PathBuf::from("photo.jpg"),
                MediaKind::Image,
                digest('a'),
            )
            .expect("valid media input")],
        )
        .expect("valid video generation request")
    }

    #[tokio::test]
    async fn fake_video_provider_progresses_submitted_running_completed() {
        let provider = FakeVideoGenerationProvider::new(FakeGenerationScenario::Scripted(vec![
            FakeGenerationStep::Submitted,
            FakeGenerationStep::Running,
            FakeGenerationStep::Completed {
                locator: "fixture://generated.mp4".to_owned(),
            },
        ]));
        let handle = provider
            .submit(&video_request())
            .await
            .expect("fake submission should succeed");

        assert_eq!(
            provider.poll(&handle).await.expect("submitted status"),
            GenerationProviderStatus::Submitted
        );
        assert_eq!(
            provider.poll(&handle).await.expect("running status"),
            GenerationProviderStatus::Running
        );
        assert!(matches!(
            provider.poll(&handle).await.expect("completed status"),
            GenerationProviderStatus::Completed { .. }
        ));
    }

    #[tokio::test]
    async fn fake_video_provider_surfaces_terminal_failure() {
        let provider = FakeVideoGenerationProvider::new(FakeGenerationScenario::Scripted(vec![
            FakeGenerationStep::Running,
            FakeGenerationStep::Failed {
                message: "fixture generation failed".to_owned(),
            },
        ]));
        let handle = provider
            .submit(&video_request())
            .await
            .expect("fake submission should succeed");

        assert_eq!(
            provider.poll(&handle).await.expect("running status"),
            GenerationProviderStatus::Running
        );
        assert_eq!(
            provider.poll(&handle).await.expect("failed status"),
            GenerationProviderStatus::Failed {
                message: "fixture generation failed".to_owned()
            }
        );
    }

    #[tokio::test]
    async fn fake_video_provider_cancellation_is_terminal() {
        let provider = FakeVideoGenerationProvider::new(FakeGenerationScenario::Scripted(vec![
            FakeGenerationStep::Running,
            FakeGenerationStep::Completed {
                locator: "fixture://must-not-surface.mp4".to_owned(),
            },
        ]));
        let handle = provider
            .submit(&video_request())
            .await
            .expect("fake submission should succeed");
        assert_eq!(
            provider.poll(&handle).await.expect("running status"),
            GenerationProviderStatus::Running
        );

        assert_eq!(
            provider.cancel(&handle).await.expect("cancel should succeed"),
            GenerationProviderStatus::Cancelled
        );
        assert_eq!(
            provider.poll(&handle).await.expect("cancel remains terminal"),
            GenerationProviderStatus::Cancelled
        );
    }

    #[tokio::test]
    async fn malformed_completion_is_rejected() {
        let provider = FakeVideoGenerationProvider::new(FakeGenerationScenario::Scripted(vec![
            FakeGenerationStep::MalformedCompletion,
        ]));
        let handle = provider
            .submit(&video_request())
            .await
            .expect("fake submission should succeed");

        let error = provider
            .poll(&handle)
            .await
            .expect_err("malformed completion must fail");
        assert_eq!(error.kind(), GenerationProviderErrorKind::MalformedResponse);
    }

    #[tokio::test]
    async fn completed_status_with_empty_locator_is_rejected() {
        let provider = FakeVideoGenerationProvider::new(FakeGenerationScenario::Scripted(vec![
            FakeGenerationStep::Completed {
                locator: String::new(),
            },
        ]));
        let handle = provider
            .submit(&video_request())
            .await
            .expect("fake submission should succeed");

        let error = provider
            .poll(&handle)
            .await
            .expect_err("empty output locator must fail");
        assert_eq!(error.kind(), GenerationProviderErrorKind::MalformedResponse);
    }

    #[tokio::test]
    async fn fake_submission_error_is_explicit() {
        let provider = FakeVideoGenerationProvider::new(FakeGenerationScenario::SubmissionError {
            kind: GenerationProviderErrorKind::Unavailable,
            message: "fixture provider unavailable".to_owned(),
        });

        let error = provider
            .submit(&video_request())
            .await
            .expect_err("submission should fail explicitly");
        assert_eq!(error.kind(), GenerationProviderErrorKind::Unavailable);
    }

    #[tokio::test]
    async fn image_provider_uses_same_vendor_neutral_lifecycle() {
        let provider = FakeImageGenerationProvider::new(FakeGenerationScenario::Scripted(vec![
            FakeGenerationStep::Completed {
                locator: "fixture://generated.png".to_owned(),
            },
        ]));
        let request = ImageGenerationRequest::new("Create a clean title card", Vec::new())
            .expect("valid image generation request");
        let handle = provider
            .submit(&request)
            .await
            .expect("fake submission should succeed");

        assert!(matches!(
            provider.poll(&handle).await.expect("completed status"),
            GenerationProviderStatus::Completed { .. }
        ));
    }

    #[test]
    fn generation_requests_reject_empty_instruction_and_invalid_source_hash() {
        assert!(VideoGenerationRequest::new("  ", Vec::new()).is_err());
        assert!(
            GenerationMediaInput::new(PathBuf::from("photo.jpg"), MediaKind::Image, "bad-hash")
                .is_err()
        );
    }
}
