//! Runway Dev adapter for Laputa's functional video-generation provider contract.
//!
//! Vendor HTTP shapes, model defaults, upload resources and API credentials stay in
//! this crate. Laputa core and Tool schemas only see `VideoGenerationProvider`.

use std::{
    collections::BTreeMap,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use async_trait::async_trait;
use laputa_domain::{GenerationProviderOutput, MediaKind, ProviderProvenance};
use laputa_provider::{
    GenerationMediaInput, GenerationProviderError, GenerationProviderErrorKind,
    GenerationProviderHandle, GenerationProviderStatus, VideoGenerationProvider,
    VideoGenerationRequest,
};
use reqwest::{Client, Method, Response, StatusCode, header::CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::time::sleep;

const DEFAULT_BASE_URL: &str = "https://api.dev.runwayml.com";
const RUNWAY_API_VERSION: &str = "2024-11-06";
const DEFAULT_MODEL: &str = "gen4.5";
const DEFAULT_RATIO: &str = "1280:720";
const DEFAULT_DURATION_SECONDS: u8 = 5;
const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(5);
const MIN_EPHEMERAL_UPLOAD_BYTES: u64 = 512;
const MAX_EPHEMERAL_UPLOAD_BYTES: u64 = 200 * 1024 * 1024;
const PROVIDER_NAME: &str = "runway-video-generation";

#[derive(Clone, Debug)]
pub struct RunwayConfig {
    api_key: String,
    model: String,
    ratio: String,
    duration_seconds: u8,
    base_url: String,
    poll_interval: Duration,
}

impl RunwayConfig {
    pub fn new(api_key: impl Into<String>) -> Result<Self, GenerationProviderError> {
        let api_key = api_key.into();
        if api_key.trim().is_empty() {
            return Err(rejected("Runway API key must not be empty"));
        }
        Ok(Self {
            api_key,
            model: DEFAULT_MODEL.to_owned(),
            ratio: DEFAULT_RATIO.to_owned(),
            duration_seconds: DEFAULT_DURATION_SECONDS,
            base_url: DEFAULT_BASE_URL.to_owned(),
            poll_interval: DEFAULT_POLL_INTERVAL,
        })
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Result<Self, GenerationProviderError> {
        let model = model.into();
        if model.trim().is_empty() {
            return Err(rejected("Runway model must not be empty"));
        }
        self.model = model;
        Ok(self)
    }

    pub fn with_ratio(mut self, ratio: impl Into<String>) -> Result<Self, GenerationProviderError> {
        let ratio = ratio.into();
        if ratio.trim().is_empty() {
            return Err(rejected("Runway ratio must not be empty"));
        }
        self.ratio = ratio;
        Ok(self)
    }

    pub fn with_duration_seconds(
        mut self,
        duration_seconds: u8,
    ) -> Result<Self, GenerationProviderError> {
        if duration_seconds == 0 {
            return Err(rejected("Runway duration must be greater than zero"));
        }
        self.duration_seconds = duration_seconds;
        Ok(self)
    }

    pub fn with_base_url(
        mut self,
        base_url: impl Into<String>,
    ) -> Result<Self, GenerationProviderError> {
        let base_url = base_url.into().trim_end_matches('/').to_owned();
        if base_url.trim().is_empty() {
            return Err(rejected("Runway base URL must not be empty"));
        }
        self.base_url = base_url;
        Ok(self)
    }
}

#[derive(Clone, Debug)]
pub struct RunwayVideoGenerationProvider {
    client: Client,
    config: RunwayConfig,
}

impl RunwayVideoGenerationProvider {
    pub fn new(config: RunwayConfig) -> Result<Self, GenerationProviderError> {
        let client = Client::builder().build().map_err(|error| {
            unavailable(format!("failed to create Runway HTTP client: {error}"))
        })?;
        Ok(Self { client, config })
    }

    async fn upload_ephemeral(
        &self,
        input: &GenerationMediaInput,
    ) -> Result<String, GenerationProviderError> {
        let path = input.path();
        let metadata = tokio::fs::metadata(path)
            .await
            .map_err(|error| rejected(format!("cannot inspect Runway upload input: {error}")))?;
        if !metadata.is_file() {
            return Err(rejected("Runway upload input is not a regular file"));
        }
        if metadata.len() < MIN_EPHEMERAL_UPLOAD_BYTES {
            return Err(rejected(format!(
                "Runway ephemeral upload must be at least {MIN_EPHEMERAL_UPLOAD_BYTES} bytes"
            )));
        }
        if metadata.len() > MAX_EPHEMERAL_UPLOAD_BYTES {
            return Err(rejected(format!(
                "Runway ephemeral upload exceeds {MAX_EPHEMERAL_UPLOAD_BYTES} bytes"
            )));
        }
        let filename = path
            .file_name()
            .and_then(|value| value.to_str())
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| rejected("Runway upload input has no UTF-8 filename"))?;

        let response = self
            .request(Method::POST, "/v1/uploads")
            .json(&UploadStartRequest {
                filename,
                kind: "ephemeral",
            })
            .send()
            .await
            .map_err(|error| {
                unavailable(format!("Runway upload initialization failed: {error}"))
            })?;
        let response = checked_response(response, "Runway upload initialization", false).await?;
        let upload: UploadStartResponse = response
            .json()
            .await
            .map_err(|error| malformed(format!("invalid Runway upload response: {error}")))?;
        validate_upload_response(&upload)?;

        let bytes = tokio::fs::read(path)
            .await
            .map_err(|error| rejected(format!("cannot read Runway upload input: {error}")))?;
        let boundary = multipart_boundary();
        let body = multipart_body(&upload.fields, filename, &bytes, &boundary)?;
        let response = self
            .client
            .post(&upload.upload_url)
            .header(
                CONTENT_TYPE,
                format!("multipart/form-data; boundary={boundary}"),
            )
            .body(body)
            .send()
            .await
            .map_err(|error| unavailable(format!("Runway media upload failed: {error}")))?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(unavailable(format!(
                "Runway media upload returned HTTP {status}: {}",
                concise_body(&body)
            )));
        }

        Ok(upload.runway_uri)
    }

    fn request(&self, method: Method, path: &str) -> reqwest::RequestBuilder {
        self.client
            .request(method, format!("{}{}", self.config.base_url, path))
            .bearer_auth(&self.config.api_key)
            .header("X-Runway-Version", RUNWAY_API_VERSION)
    }

    async fn create_task(
        &self,
        endpoint: &str,
        payload: &Value,
    ) -> Result<GenerationProviderHandle, GenerationProviderError> {
        let response = self
            .request(Method::POST, endpoint)
            .json(payload)
            .send()
            .await
            .map_err(|error| unavailable(format!("Runway task submission failed: {error}")))?;
        let response = checked_response(response, "Runway task submission", false).await?;
        let task: TaskCreated = response
            .json()
            .await
            .map_err(|error| malformed(format!("invalid Runway task response: {error}")))?;
        if task.id.trim().is_empty() {
            return Err(malformed("Runway task response omitted task id"));
        }

        GenerationProviderHandle::new(ProviderProvenance {
            provider: PROVIDER_NAME.to_owned(),
            request_id: None,
            job_id: Some(task.id),
        })
    }
}

#[async_trait]
impl VideoGenerationProvider for RunwayVideoGenerationProvider {
    async fn submit(
        &self,
        request: &VideoGenerationRequest,
    ) -> Result<GenerationProviderHandle, GenerationProviderError> {
        let image_input = selected_image_input(request)?;
        match image_input {
            Some(input) => {
                let runway_uri = self.upload_ephemeral(input).await?;
                let payload =
                    generation_payload(&self.config, request.instruction(), Some(&runway_uri));
                self.create_task("/v1/image_to_video", &payload).await
            }
            None => {
                let payload = generation_payload(&self.config, request.instruction(), None);
                self.create_task("/v1/text_to_video", &payload).await
            }
        }
    }

    async fn poll(
        &self,
        handle: &GenerationProviderHandle,
    ) -> Result<GenerationProviderStatus, GenerationProviderError> {
        sleep(self.config.poll_interval).await;
        let task_id = provider_job_id(handle)?;
        let response = self
            .request(Method::GET, &format!("/v1/tasks/{task_id}"))
            .send()
            .await
            .map_err(|error| unavailable(format!("Runway task polling failed: {error}")))?;
        let response = checked_response(response, "Runway task polling", true).await?;
        let task: TaskDetail = response
            .json()
            .await
            .map_err(|error| malformed(format!("invalid Runway task detail: {error}")))?;
        if task.id != task_id {
            return Err(malformed(
                "Runway task detail id does not match requested task",
            ));
        }
        map_task_status(task)
    }

    async fn cancel(
        &self,
        handle: &GenerationProviderHandle,
    ) -> Result<GenerationProviderStatus, GenerationProviderError> {
        let task_id = provider_job_id(handle)?;
        let response = self
            .request(Method::DELETE, &format!("/v1/tasks/{task_id}"))
            .send()
            .await
            .map_err(|error| unavailable(format!("Runway task cancellation failed: {error}")))?;
        checked_response(response, "Runway task cancellation", true).await?;
        Ok(GenerationProviderStatus::Cancelled)
    }
}

fn selected_image_input(
    request: &VideoGenerationRequest,
) -> Result<Option<&GenerationMediaInput>, GenerationProviderError> {
    match request.inputs() {
        [] => Ok(None),
        [input] if input.media_kind() == MediaKind::Image => Ok(Some(input)),
        [_] => Err(rejected(
            "Runway selected video subset accepts only an image source",
        )),
        _ => Err(rejected(
            "Runway selected video subset accepts at most one image source",
        )),
    }
}

fn generation_payload(config: &RunwayConfig, instruction: &str, image: Option<&str>) -> Value {
    let mut payload = json!({
        "model": &config.model,
        "promptText": instruction,
        "ratio": &config.ratio,
        "duration": config.duration_seconds
    });
    if let Some(image) = image {
        payload["promptImage"] = Value::String(image.to_owned());
    }
    payload
}

fn provider_job_id(handle: &GenerationProviderHandle) -> Result<&str, GenerationProviderError> {
    handle
        .provenance()
        .job_id
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| malformed("Runway provider handle omitted task id"))
}

fn map_task_status(task: TaskDetail) -> Result<GenerationProviderStatus, GenerationProviderError> {
    match task.status.as_str() {
        "PENDING" | "THROTTLED" => Ok(GenerationProviderStatus::Submitted),
        "RUNNING" => Ok(GenerationProviderStatus::Running),
        "SUCCEEDED" => {
            let locator = task
                .output
                .into_iter()
                .find(|value| !value.trim().is_empty())
                .ok_or_else(|| malformed("Runway succeeded task returned no output URL"))?;
            if !locator.starts_with("https://") {
                return Err(malformed("Runway succeeded task output must be HTTPS"));
            }
            Ok(GenerationProviderStatus::Completed {
                output: GenerationProviderOutput { locator },
            })
        }
        "FAILED" => Ok(GenerationProviderStatus::Failed {
            message: task_failure_message(&task),
        }),
        "CANCELED" | "CANCELLED" => Ok(GenerationProviderStatus::Cancelled),
        status => Err(malformed(format!(
            "Runway returned unknown task status {status}"
        ))),
    }
}

fn task_failure_message(task: &TaskDetail) -> String {
    match (
        task.failure_code
            .as_deref()
            .filter(|value| !value.trim().is_empty()),
        task.failure
            .as_deref()
            .filter(|value| !value.trim().is_empty()),
    ) {
        (Some(code), Some(message)) => format!("{code}: {message}"),
        (Some(code), None) => code.to_owned(),
        (None, Some(message)) => message.to_owned(),
        (None, None) => "Runway task failed without details".to_owned(),
    }
}

async fn checked_response(
    response: Response,
    operation: &str,
    unknown_job_on_not_found: bool,
) -> Result<Response, GenerationProviderError> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }
    let body = response.text().await.unwrap_or_default();
    let message = format!(
        "{operation} returned HTTP {status}: {}",
        concise_body(&body)
    );
    let kind = if unknown_job_on_not_found && status == StatusCode::NOT_FOUND {
        GenerationProviderErrorKind::UnknownJob
    } else if status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error() {
        GenerationProviderErrorKind::Unavailable
    } else {
        GenerationProviderErrorKind::Rejected
    };
    Err(GenerationProviderError::new(kind, message))
}

fn validate_upload_response(upload: &UploadStartResponse) -> Result<(), GenerationProviderError> {
    if !upload.upload_url.starts_with("https://") {
        return Err(malformed("Runway upload response omitted HTTPS uploadUrl"));
    }
    if !upload.runway_uri.starts_with("runway://") {
        return Err(malformed("Runway upload response omitted runway:// URI"));
    }
    Ok(())
}

fn multipart_boundary() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!("laputa-runway-{nanos}")
}

fn multipart_body(
    fields: &BTreeMap<String, String>,
    filename: &str,
    file_bytes: &[u8],
    boundary: &str,
) -> Result<Vec<u8>, GenerationProviderError> {
    validate_disposition_value(filename, "Runway upload filename")?;
    validate_disposition_value(boundary, "Runway multipart boundary")?;
    let mut body = Vec::new();
    for (name, value) in fields {
        validate_disposition_value(name, "Runway upload form field name")?;
        append_text(&mut body, &format!("--{boundary}\r\n"));
        append_text(
            &mut body,
            &format!("Content-Disposition: form-data; name=\"{name}\"\r\n\r\n"),
        );
        append_text(&mut body, value);
        append_text(&mut body, "\r\n");
    }
    append_text(&mut body, &format!("--{boundary}\r\n"));
    append_text(
        &mut body,
        &format!("Content-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\n"),
    );
    append_text(&mut body, "Content-Type: application/octet-stream\r\n\r\n");
    body.extend_from_slice(file_bytes);
    append_text(&mut body, "\r\n");
    append_text(&mut body, &format!("--{boundary}--\r\n"));
    Ok(body)
}

fn validate_disposition_value(value: &str, field: &str) -> Result<(), GenerationProviderError> {
    if value
        .chars()
        .any(|character| matches!(character, '\r' | '\n' | '"'))
    {
        return Err(rejected(format!("{field} contains unsafe characters")));
    }
    Ok(())
}

fn append_text(buffer: &mut Vec<u8>, text: &str) {
    buffer.extend_from_slice(text.as_bytes());
}

fn concise_body(body: &str) -> String {
    const MAX_CHARS: usize = 512;
    let trimmed = body.trim();
    let mut chars = trimmed.chars();
    let text: String = chars.by_ref().take(MAX_CHARS).collect();
    if chars.next().is_some() {
        format!("{text}…")
    } else if text.is_empty() {
        "<empty response>".to_owned()
    } else {
        text
    }
}

fn rejected(message: impl Into<String>) -> GenerationProviderError {
    GenerationProviderError::new(GenerationProviderErrorKind::Rejected, message)
}

fn unavailable(message: impl Into<String>) -> GenerationProviderError {
    GenerationProviderError::new(GenerationProviderErrorKind::Unavailable, message)
}

fn malformed(message: impl Into<String>) -> GenerationProviderError {
    GenerationProviderError::new(GenerationProviderErrorKind::MalformedResponse, message)
}

#[derive(Serialize)]
struct UploadStartRequest<'a> {
    filename: &'a str,
    #[serde(rename = "type")]
    kind: &'static str,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UploadStartResponse {
    upload_url: String,
    #[serde(default)]
    fields: BTreeMap<String, String>,
    runway_uri: String,
}

#[derive(Debug, Deserialize)]
struct TaskCreated {
    id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TaskDetail {
    id: String,
    status: String,
    #[serde(default)]
    output: Vec<String>,
    failure: Option<String>,
    failure_code: Option<String>,
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, path::PathBuf};

    use laputa_domain::{MediaKind, ProviderProvenance};
    use laputa_provider::{
        GenerationMediaInput, GenerationProviderErrorKind, GenerationProviderHandle,
        GenerationProviderStatus, VideoGenerationRequest,
    };

    use super::{
        RunwayConfig, TaskDetail, generation_payload, map_task_status, multipart_body,
        provider_job_id, selected_image_input,
    };

    fn digest(character: char) -> String {
        std::iter::repeat_n(character, 64).collect()
    }

    #[test]
    fn config_defaults_keep_vendor_controls_inside_adapter() {
        let config = RunwayConfig::new("fixture-secret").expect("config");
        let payload = generation_payload(&config, "A quiet sunrise over Kyoto", None);

        assert_eq!(payload["model"], "gen4.5");
        assert_eq!(payload["ratio"], "1280:720");
        assert_eq!(payload["duration"], 5);
        assert_eq!(payload["promptText"], "A quiet sunrise over Kyoto");
        assert!(payload.get("promptImage").is_none());
    }

    #[test]
    fn image_request_payload_uses_runway_upload_uri() {
        let config = RunwayConfig::new("fixture-secret").expect("config");
        let payload = generation_payload(
            &config,
            "Subtle cinematic push-in",
            Some("runway://fixture-image"),
        );

        assert_eq!(payload["promptImage"], "runway://fixture-image");
        assert_eq!(payload["promptText"], "Subtle cinematic push-in");
    }

    #[test]
    fn selected_subset_accepts_zero_or_one_image_input_only() {
        let text =
            VideoGenerationRequest::new("Generate B-roll", Vec::new()).expect("text request");
        assert!(selected_image_input(&text).expect("text input").is_none());

        let image =
            GenerationMediaInput::new(PathBuf::from("photo.jpg"), MediaKind::Image, digest('a'))
                .expect("image input");
        let image_request =
            VideoGenerationRequest::new("Animate", vec![image]).expect("image request");
        assert!(
            selected_image_input(&image_request)
                .expect("image selected")
                .is_some()
        );

        let video =
            GenerationMediaInput::new(PathBuf::from("clip.mp4"), MediaKind::Video, digest('b'))
                .expect("video input");
        let video_request =
            VideoGenerationRequest::new("Transform", vec![video]).expect("video request");
        let error = selected_image_input(&video_request).expect_err("video input rejected");
        assert_eq!(error.kind(), GenerationProviderErrorKind::Rejected);
    }

    #[test]
    fn task_status_mapping_preserves_async_semantics() {
        for status in ["PENDING", "THROTTLED"] {
            assert_eq!(
                map_task_status(task(status, Vec::new())).expect("queued"),
                GenerationProviderStatus::Submitted
            );
        }
        assert_eq!(
            map_task_status(task("RUNNING", Vec::new())).expect("running"),
            GenerationProviderStatus::Running
        );
        let completed = map_task_status(task(
            "SUCCEEDED",
            vec!["https://example.invalid/output.mp4".to_owned()],
        ))
        .expect("completed");
        assert!(matches!(
            completed,
            GenerationProviderStatus::Completed { .. }
        ));
        assert_eq!(
            map_task_status(task("CANCELED", Vec::new())).expect("cancelled"),
            GenerationProviderStatus::Cancelled
        );
    }

    #[test]
    fn succeeded_task_requires_https_output() {
        let missing = map_task_status(task("SUCCEEDED", Vec::new()))
            .expect_err("missing output must be malformed");
        assert_eq!(
            missing.kind(),
            GenerationProviderErrorKind::MalformedResponse
        );

        let insecure = map_task_status(task(
            "SUCCEEDED",
            vec!["http://example.invalid/output.mp4".to_owned()],
        ))
        .expect_err("http output must be malformed");
        assert_eq!(
            insecure.kind(),
            GenerationProviderErrorKind::MalformedResponse
        );
    }

    #[test]
    fn failed_task_keeps_failure_code_and_message() {
        let mut detail = task("FAILED", Vec::new());
        detail.failure_code = Some("SAFETY.INPUT.TEXT".to_owned());
        detail.failure = Some("prompt was rejected".to_owned());

        assert_eq!(
            map_task_status(detail).expect("failed status"),
            GenerationProviderStatus::Failed {
                message: "SAFETY.INPUT.TEXT: prompt was rejected".to_owned()
            }
        );
    }

    #[test]
    fn multipart_upload_contains_presigned_fields_and_file() {
        let mut fields = BTreeMap::new();
        fields.insert("key".to_owned(), "uploads/fixture".to_owned());
        fields.insert("policy".to_owned(), "signed-policy".to_owned());
        let body = multipart_body(
            &fields,
            "photo.jpg",
            b"fixture-image-bytes",
            "fixture-boundary",
        )
        .expect("multipart body");
        let body = String::from_utf8_lossy(&body);

        assert!(body.contains("name=\"key\""));
        assert!(body.contains("uploads/fixture"));
        assert!(body.contains("name=\"policy\""));
        assert!(body.contains("filename=\"photo.jpg\""));
        assert!(body.contains("fixture-image-bytes"));
        assert!(body.ends_with("--fixture-boundary--\r\n"));
    }

    #[test]
    fn provider_handle_requires_durable_runway_task_id() {
        let handle = GenerationProviderHandle::new(ProviderProvenance {
            provider: "runway-video-generation".to_owned(),
            request_id: None,
            job_id: Some("task-123".to_owned()),
        })
        .expect("handle");

        assert_eq!(provider_job_id(&handle).expect("job id"), "task-123");
    }

    fn task(status: &str, output: Vec<String>) -> TaskDetail {
        TaskDetail {
            id: "task-123".to_owned(),
            status: status.to_owned(),
            output,
            failure: None,
            failure_code: None,
        }
    }
}
