use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use async_trait::async_trait;
use laputa_domain::GenerationProviderOutput;
use laputa_provider::{
    GenerationProviderError, GenerationProviderErrorKind, GenerationProviderHandle,
    GenerationProviderStatus, VideoGenerationProvider, VideoGenerationRequest,
};
use reqwest::Client;

const RUNWAY_STAGING_DIRECTORY: &str = ".laputa/provider-staging/runway";
const MAX_REMOTE_OUTPUT_BYTES: u64 = 512 * 1024 * 1024;
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[async_trait]
pub trait RunwayOutputFetcher: Send + Sync {
    async fn fetch(&self, locator: &str, destination: &Path)
    -> Result<(), GenerationProviderError>;
}

#[derive(Clone, Debug)]
pub struct ReqwestRunwayOutputFetcher {
    client: Client,
}

impl ReqwestRunwayOutputFetcher {
    pub fn new() -> Result<Self, GenerationProviderError> {
        let client = Client::builder().build().map_err(|error| {
            unavailable(format!(
                "failed to create Runway output HTTP client: {error}"
            ))
        })?;
        Ok(Self { client })
    }
}

#[async_trait]
impl RunwayOutputFetcher for ReqwestRunwayOutputFetcher {
    async fn fetch(
        &self,
        locator: &str,
        destination: &Path,
    ) -> Result<(), GenerationProviderError> {
        ensure_https(locator)?;
        let parent = destination
            .parent()
            .ok_or_else(|| unavailable("Runway staging destination has no parent directory"))?;
        fs::create_dir_all(parent).map_err(|error| {
            unavailable(format!(
                "failed to create Runway project staging directory {}: {error}",
                parent.display()
            ))
        })?;
        let temporary = temporary_path(destination);
        remove_if_exists(&temporary);

        let mut response = self.client.get(locator).send().await.map_err(|error| {
            unavailable(format!("Runway output download failed to start: {error}"))
        })?;
        if !response.status().is_success() {
            let status = response.status();
            remove_if_exists(&temporary);
            return Err(unavailable(format!(
                "Runway output download returned HTTP {status}"
            )));
        }
        if response
            .content_length()
            .is_some_and(|length| length > MAX_REMOTE_OUTPUT_BYTES)
        {
            remove_if_exists(&temporary);
            return Err(rejected(format!(
                "Runway output exceeds the {MAX_REMOTE_OUTPUT_BYTES}-byte staging limit"
            )));
        }

        let mut file = File::create(&temporary).map_err(|error| {
            unavailable(format!(
                "failed to create Runway staging file {}: {error}",
                temporary.display()
            ))
        })?;
        let mut written = 0_u64;
        loop {
            let chunk = response.chunk().await.map_err(|error| {
                remove_if_exists(&temporary);
                unavailable(format!("Runway output download failed: {error}"))
            })?;
            let Some(chunk) = chunk else {
                break;
            };
            written = written.saturating_add(chunk.len() as u64);
            if written > MAX_REMOTE_OUTPUT_BYTES {
                drop(file);
                remove_if_exists(&temporary);
                return Err(rejected(format!(
                    "Runway output exceeds the {MAX_REMOTE_OUTPUT_BYTES}-byte staging limit"
                )));
            }
            file.write_all(&chunk).map_err(|error| {
                remove_if_exists(&temporary);
                unavailable(format!("failed to write Runway staging output: {error}"))
            })?;
        }
        if written == 0 {
            drop(file);
            remove_if_exists(&temporary);
            return Err(malformed("Runway output download was empty"));
        }
        file.sync_all().map_err(|error| {
            remove_if_exists(&temporary);
            unavailable(format!("failed to sync Runway staging output: {error}"))
        })?;
        drop(file);
        install_replacement(&temporary, destination)?;
        Ok(())
    }
}

pub struct RunwayProjectVideoGenerationProvider<P, F = ReqwestRunwayOutputFetcher> {
    inner: P,
    fetcher: F,
    project_root: PathBuf,
}

impl<P> RunwayProjectVideoGenerationProvider<P, ReqwestRunwayOutputFetcher> {
    pub fn new(
        inner: P,
        project_root: impl Into<PathBuf>,
    ) -> Result<Self, GenerationProviderError> {
        Self::with_fetcher(inner, project_root, ReqwestRunwayOutputFetcher::new()?)
    }
}

impl<P, F> RunwayProjectVideoGenerationProvider<P, F> {
    pub fn with_fetcher(
        inner: P,
        project_root: impl Into<PathBuf>,
        fetcher: F,
    ) -> Result<Self, GenerationProviderError> {
        let project_root = project_root.into();
        if project_root.as_os_str().is_empty() {
            return Err(rejected("Runway project root must not be empty"));
        }
        Ok(Self {
            inner,
            fetcher,
            project_root,
        })
    }

    #[must_use]
    pub fn inner(&self) -> &P {
        &self.inner
    }

    fn staging_path(
        &self,
        handle: &GenerationProviderHandle,
    ) -> Result<PathBuf, GenerationProviderError> {
        let task_id = provider_task_component(handle)?;
        Ok(self
            .project_root
            .join(RUNWAY_STAGING_DIRECTORY)
            .join(task_id)
            .join("output.mp4"))
    }
}

#[async_trait]
impl<P, F> VideoGenerationProvider for RunwayProjectVideoGenerationProvider<P, F>
where
    P: VideoGenerationProvider,
    F: RunwayOutputFetcher,
{
    async fn submit(
        &self,
        request: &VideoGenerationRequest,
    ) -> Result<GenerationProviderHandle, GenerationProviderError> {
        self.inner.submit(request).await
    }

    async fn poll(
        &self,
        handle: &GenerationProviderHandle,
    ) -> Result<GenerationProviderStatus, GenerationProviderError> {
        match self.inner.poll(handle).await? {
            GenerationProviderStatus::Completed { output } => {
                output.validate().map_err(|error| {
                    GenerationProviderError::new(
                        GenerationProviderErrorKind::MalformedResponse,
                        error.to_string(),
                    )
                })?;
                if !output.locator.starts_with("https://") {
                    return Ok(GenerationProviderStatus::Completed { output });
                }
                let destination = self.staging_path(handle)?;
                let reusable = fs::metadata(&destination)
                    .map(|metadata| metadata.is_file() && metadata.len() > 0)
                    .unwrap_or(false);
                if !reusable {
                    self.fetcher.fetch(&output.locator, &destination).await?;
                }
                let metadata = fs::metadata(&destination).map_err(|error| {
                    unavailable(format!(
                        "failed to inspect Runway project-owned staging output {}: {error}",
                        destination.display()
                    ))
                })?;
                if !metadata.is_file() || metadata.len() == 0 {
                    return Err(malformed(
                        "Runway project-owned staging output is missing or empty",
                    ));
                }
                Ok(GenerationProviderStatus::Completed {
                    output: GenerationProviderOutput {
                        locator: destination.to_string_lossy().into_owned(),
                    },
                })
            }
            status => Ok(status),
        }
    }

    async fn cancel(
        &self,
        handle: &GenerationProviderHandle,
    ) -> Result<GenerationProviderStatus, GenerationProviderError> {
        match self.inner.cancel(handle).await {
            Err(error) if error.kind() == GenerationProviderErrorKind::UnknownJob => {
                Ok(GenerationProviderStatus::Cancelled)
            }
            result => result,
        }
    }
}

fn provider_task_component(
    handle: &GenerationProviderHandle,
) -> Result<&str, GenerationProviderError> {
    let task_id = handle
        .provenance()
        .job_id
        .as_deref()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| malformed("Runway project wrapper handle omitted task id"))?;
    if !task_id
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(malformed(
            "Runway task id contains characters unsafe for project staging",
        ));
    }
    Ok(task_id)
}

fn ensure_https(locator: &str) -> Result<(), GenerationProviderError> {
    if locator.starts_with("https://") {
        Ok(())
    } else {
        Err(malformed("Runway remote output locator must be HTTPS"))
    }
}

fn temporary_path(destination: &Path) -> PathBuf {
    let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let name = destination
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("output.mp4");
    destination.with_file_name(format!(".{name}.tmp-{}-{sequence}", std::process::id()))
}

fn install_replacement(
    temporary: &Path,
    destination: &Path,
) -> Result<(), GenerationProviderError> {
    match fs::rename(temporary, destination) {
        Ok(()) => Ok(()),
        Err(error) if destination.exists() => {
            fs::remove_file(destination).map_err(|remove_error| {
                unavailable(format!(
                    "failed to replace Runway staging output after rename error {error}: {remove_error}"
                ))
            })?;
            fs::rename(temporary, destination).map_err(|rename_error| {
                unavailable(format!(
                    "failed to install Runway project staging output: {rename_error}"
                ))
            })
        }
        Err(error) => Err(unavailable(format!(
            "failed to install Runway project staging output: {error}"
        ))),
    }
}

fn remove_if_exists(path: &Path) {
    if path.exists() {
        let _ = fs::remove_file(path);
    }
}

fn unavailable(message: impl Into<String>) -> GenerationProviderError {
    GenerationProviderError::new(GenerationProviderErrorKind::Unavailable, message)
}

fn rejected(message: impl Into<String>) -> GenerationProviderError {
    GenerationProviderError::new(GenerationProviderErrorKind::Rejected, message)
}

fn malformed(message: impl Into<String>) -> GenerationProviderError {
    GenerationProviderError::new(GenerationProviderErrorKind::MalformedResponse, message)
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        time::{SystemTime, UNIX_EPOCH},
    };

    use async_trait::async_trait;
    use laputa_domain::ProviderProvenance;
    use laputa_provider::{
        FakeGenerationScenario, FakeGenerationStep, FakeVideoGenerationProvider,
        GenerationProviderError, GenerationProviderErrorKind, GenerationProviderHandle,
        GenerationProviderStatus, VideoGenerationProvider, VideoGenerationRequest,
    };

    use super::{
        RunwayOutputFetcher, RunwayProjectVideoGenerationProvider, provider_task_component,
    };

    struct CopyFetcher {
        source: PathBuf,
        calls: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl RunwayOutputFetcher for CopyFetcher {
        async fn fetch(
            &self,
            _locator: &str,
            destination: &Path,
        ) -> Result<(), GenerationProviderError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            let parent = destination.parent().ok_or_else(|| {
                GenerationProviderError::new(
                    GenerationProviderErrorKind::Unavailable,
                    "fixture destination has no parent",
                )
            })?;
            fs::create_dir_all(parent).map_err(|error| {
                GenerationProviderError::new(
                    GenerationProviderErrorKind::Unavailable,
                    error.to_string(),
                )
            })?;
            fs::copy(&self.source, destination).map_err(|error| {
                GenerationProviderError::new(
                    GenerationProviderErrorKind::Unavailable,
                    error.to_string(),
                )
            })?;
            Ok(())
        }
    }

    fn test_root(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        std::env::temp_dir().join(format!(
            "laputa-runway-project-{label}-{}-{nanos}",
            std::process::id()
        ))
    }

    #[tokio::test]
    async fn completed_https_output_is_staged_inside_project_and_reused() {
        let root = test_root("stage");
        fs::create_dir_all(&root).expect("fixture root should be created");
        let source = root.join("remote-source.mp4");
        fs::write(&source, b"fixture-video-bytes").expect("fixture source should be written");
        let calls = Arc::new(AtomicUsize::new(0));
        let inner = FakeVideoGenerationProvider::new(FakeGenerationScenario::Scripted(vec![
            FakeGenerationStep::Completed {
                locator: "https://cdn.example.test/output.mp4".to_owned(),
            },
        ]));
        let provider = RunwayProjectVideoGenerationProvider::with_fetcher(
            inner,
            &root,
            CopyFetcher {
                source: source.clone(),
                calls: calls.clone(),
            },
        )
        .expect("project wrapper should initialize");
        let request = VideoGenerationRequest::new("fixture B-roll", Vec::new())
            .expect("fixture request should be valid");
        let handle = provider
            .submit(&request)
            .await
            .expect("fixture submission should succeed");

        let first = provider
            .poll(&handle)
            .await
            .expect("first completed poll should stage output");
        let second = provider
            .poll(&handle)
            .await
            .expect("second completed poll should reuse staged output");

        let locator = match first {
            GenerationProviderStatus::Completed { output } => output.locator,
            status => panic!("expected completed output, got {status:?}"),
        };
        assert!(Path::new(&locator).starts_with(&root));
        assert_eq!(
            fs::read(&locator).expect("staged output should exist"),
            b"fixture-video-bytes"
        );
        assert!(matches!(second, GenerationProviderStatus::Completed { .. }));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn task_component_rejects_path_traversal_characters() {
        let handle = GenerationProviderHandle::new(ProviderProvenance {
            provider: "runway-video-generation".to_owned(),
            request_id: None,
            job_id: Some("../../escape".to_owned()),
        })
        .expect("provider handle shape itself should be valid");

        let error = provider_task_component(&handle).expect_err("unsafe task id must be rejected");
        assert_eq!(error.kind(), GenerationProviderErrorKind::MalformedResponse);
    }

    #[test]
    fn empty_project_root_is_rejected() {
        let inner = FakeVideoGenerationProvider::new(FakeGenerationScenario::Scripted(Vec::new()));
        let result = RunwayProjectVideoGenerationProvider::with_fetcher(
            inner,
            PathBuf::new(),
            CopyFetcher {
                source: PathBuf::from("unused"),
                calls: Arc::new(AtomicUsize::new(0)),
            },
        );
        assert!(result.is_err());
    }
}
