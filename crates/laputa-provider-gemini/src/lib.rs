//! Gemini adapter for Laputa functional provider contracts.
//!
//! Vendor HTTP shapes, upload resources, model names and API credentials are kept in
//! this crate. The rest of Laputa only sees `MediaUnderstandingProvider` and the
//! canonical `AssetAnalysis` domain type.

use std::{path::Path, time::Duration};

use async_trait::async_trait;
use laputa_domain::{Asset, AssetAnalysis, MediaKind};
use laputa_provider::{
    MediaUnderstandingError, MediaUnderstandingErrorKind, MediaUnderstandingProvider,
};
use reqwest::{Client, Response, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::time::{Instant, sleep};

const DEFAULT_BASE_URL: &str = "https://generativelanguage.googleapis.com";
const DEFAULT_PROCESSING_TIMEOUT: Duration = Duration::from_secs(120);
const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(2);

#[derive(Clone)]
pub struct GeminiConfig {
    api_key: String,
    model: String,
    base_url: String,
    processing_timeout: Duration,
    poll_interval: Duration,
}

impl GeminiConfig {
    pub fn new(
        api_key: impl Into<String>,
        model: impl Into<String>,
    ) -> Result<Self, MediaUnderstandingError> {
        let api_key = api_key.into();
        let model = model.into();
        if api_key.trim().is_empty() {
            return Err(rejected("Gemini API key must not be empty"));
        }
        if model.trim().is_empty() {
            return Err(rejected("Gemini model must not be empty"));
        }

        Ok(Self {
            api_key,
            model,
            base_url: DEFAULT_BASE_URL.to_owned(),
            processing_timeout: DEFAULT_PROCESSING_TIMEOUT,
            poll_interval: DEFAULT_POLL_INTERVAL,
        })
    }

    #[must_use]
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into().trim_end_matches('/').to_owned();
        self
    }

    #[must_use]
    pub fn with_processing_timing(mut self, timeout: Duration, poll_interval: Duration) -> Self {
        self.processing_timeout = timeout;
        self.poll_interval = poll_interval;
        self
    }
}

pub struct GeminiMediaUnderstandingProvider {
    client: Client,
    config: GeminiConfig,
}

impl GeminiMediaUnderstandingProvider {
    pub fn new(config: GeminiConfig) -> Result<Self, MediaUnderstandingError> {
        if config.base_url.trim().is_empty() {
            return Err(rejected("Gemini base URL must not be empty"));
        }
        if config.processing_timeout.is_zero() {
            return Err(rejected("Gemini processing timeout must be greater than zero"));
        }
        if config.poll_interval.is_zero() {
            return Err(rejected("Gemini poll interval must be greater than zero"));
        }

        let client = Client::builder()
            .build()
            .map_err(|error| unavailable(format!("failed to create Gemini HTTP client: {error}")))?;
        Ok(Self { client, config })
    }

    async fn upload_file(
        &self,
        asset: &Asset,
        mime_type: &str,
    ) -> Result<GeminiFile, MediaUnderstandingError> {
        let metadata = tokio::fs::metadata(&asset.original_path)
            .await
            .map_err(|error| rejected(format!("cannot read imported media metadata: {error}")))?;
        if !metadata.is_file() {
            return Err(rejected("imported media path is not a regular file"));
        }
        let byte_len = metadata.len();

        let start_url = format!("{}/upload/v1beta/files", self.config.base_url);
        let response = self
            .client
            .post(start_url)
            .header("x-goog-api-key", &self.config.api_key)
            .header("X-Goog-Upload-Protocol", "resumable")
            .header("X-Goog-Upload-Command", "start")
            .header("X-Goog-Upload-Header-Content-Length", byte_len.to_string())
            .header("X-Goog-Upload-Header-Content-Type", mime_type)
            .json(&UploadStartRequest {
                file: UploadFileMetadata {
                    display_name: &asset.display_name,
                },
            })
            .send()
            .await
            .map_err(|error| unavailable(format!("Gemini upload start failed: {error}")))?;
        let response = checked_response(response, "Gemini upload start").await?;
        let upload_url = response
            .headers()
            .get("x-goog-upload-url")
            .ok_or_else(|| malformed("Gemini upload start omitted x-goog-upload-url"))?
            .to_str()
            .map_err(|_| malformed("Gemini upload URL header is not valid UTF-8"))?
            .to_owned();

        let source = tokio::fs::File::open(&asset.original_path)
            .await
            .map_err(|error| rejected(format!("cannot open imported media for upload: {error}")))?;
        let response = self
            .client
            .post(upload_url)
            .header("Content-Length", byte_len.to_string())
            .header("X-Goog-Upload-Offset", "0")
            .header("X-Goog-Upload-Command", "upload, finalize")
            .body(reqwest::Body::from(source))
            .send()
            .await
            .map_err(|error| unavailable(format!("Gemini media upload failed: {error}")))?;
        let response = checked_response(response, "Gemini media upload").await?;
        let envelope: FileEnvelope = response
            .json()
            .await
            .map_err(|error| malformed(format!("invalid Gemini upload response: {error}")))?;
        validate_file_identity(envelope.file)
    }

    async fn wait_until_active(
        &self,
        mut file: GeminiFile,
    ) -> Result<GeminiFile, MediaUnderstandingError> {
        let deadline = Instant::now() + self.config.processing_timeout;
        loop {
            match file.state.as_deref() {
                Some("ACTIVE") => return Ok(file),
                Some("FAILED") => {
                    return Err(rejected("Gemini failed to process the uploaded media"));
                }
                Some("PROCESSING") | Some("STATE_UNSPECIFIED") | None => {}
                Some(state) => {
                    return Err(malformed(format!(
                        "Gemini returned unknown file state {state}"
                    )));
                }
            }

            if Instant::now() >= deadline {
                return Err(unavailable("Gemini media processing timed out"));
            }
            sleep(self.config.poll_interval).await;

            let url = format!("{}/v1beta/{}", self.config.base_url, file.name);
            let response = self
                .client
                .get(url)
                .header("x-goog-api-key", &self.config.api_key)
                .send()
                .await
                .map_err(|error| unavailable(format!("Gemini file polling failed: {error}")))?;
            let response = checked_response(response, "Gemini file polling").await?;
            file = response
                .json()
                .await
                .map_err(|error| malformed(format!("invalid Gemini file response: {error}")))?;
            file = validate_file_identity(file)?;
        }
    }

    async fn create_interaction(
        &self,
        asset: &Asset,
        file: &GeminiFile,
        mime_type: &str,
    ) -> Result<AssetAnalysis, MediaUnderstandingError> {
        let media_type = match asset.media_kind {
            MediaKind::Image => "image",
            MediaKind::Video => "video",
        };
        let prompt = analysis_prompt(asset);
        let request = json!({
            "model": self.config.model,
            "input": [
                {
                    "type": media_type,
                    "uri": file.uri,
                    "mime_type": mime_type
                },
                {
                    "type": "text",
                    "text": prompt
                }
            ],
            "response_format": {
                "type": "text",
                "mime_type": "application/json",
                "schema": analysis_schema(asset.media_kind)
            }
        });

        let url = format!("{}/v1beta/interactions", self.config.base_url);
        let response = self
            .client
            .post(url)
            .header("x-goog-api-key", &self.config.api_key)
            .json(&request)
            .send()
            .await
            .map_err(|error| unavailable(format!("Gemini interaction failed: {error}")))?;
        let response = checked_response(response, "Gemini interaction").await?;
        let interaction: InteractionResponse = response
            .json()
            .await
            .map_err(|error| malformed(format!("invalid Gemini interaction response: {error}")))?;
        parse_interaction_analysis(&interaction)
    }

    async fn delete_file_best_effort(&self, file: &GeminiFile) {
        let url = format!("{}/v1beta/{}", self.config.base_url, file.name);
        let _response = self
            .client
            .delete(url)
            .header("x-goog-api-key", &self.config.api_key)
            .send()
            .await;
    }
}

#[async_trait]
impl MediaUnderstandingProvider for GeminiMediaUnderstandingProvider {
    async fn analyze(&self, asset: &Asset) -> Result<AssetAnalysis, MediaUnderstandingError> {
        let mime_type = mime_type_for_asset(asset)?;
        let uploaded = self.upload_file(asset, mime_type).await?;
        let result = async {
            let active = self.wait_until_active(uploaded.clone()).await?;
            self.create_interaction(asset, &active, mime_type).await
        }
        .await;
        self.delete_file_best_effort(&uploaded).await;
        result
    }
}

#[derive(Serialize)]
struct UploadStartRequest<'a> {
    file: UploadFileMetadata<'a>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UploadFileMetadata<'a> {
    display_name: &'a str,
}

#[derive(Deserialize)]
struct FileEnvelope {
    file: GeminiFile,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiFile {
    name: String,
    uri: String,
    #[allow(dead_code)]
    mime_type: Option<String>,
    state: Option<String>,
}

#[derive(Deserialize)]
struct InteractionResponse {
    #[serde(default)]
    steps: Vec<InteractionStep>,
}

#[derive(Deserialize)]
struct InteractionStep {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    content: Vec<InteractionContent>,
}

#[derive(Deserialize)]
struct InteractionContent {
    #[serde(rename = "type")]
    kind: String,
    text: Option<String>,
}

fn validate_file_identity(file: GeminiFile) -> Result<GeminiFile, MediaUnderstandingError> {
    if file.name.trim().is_empty() {
        return Err(malformed("Gemini file response omitted resource name"));
    }
    if file.uri.trim().is_empty() {
        return Err(malformed("Gemini file response omitted file URI"));
    }
    Ok(file)
}

fn mime_type_for_asset(asset: &Asset) -> Result<&'static str, MediaUnderstandingError> {
    let extension = asset
        .original_path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .ok_or_else(|| rejected("imported media has no usable extension"))?;

    let (mime_type, kind) = match extension.as_str() {
        "jpg" | "jpeg" => ("image/jpeg", MediaKind::Image),
        "png" => ("image/png", MediaKind::Image),
        "webp" => ("image/webp", MediaKind::Image),
        "heic" => ("image/heic", MediaKind::Image),
        "heif" => ("image/heif", MediaKind::Image),
        "gif" => ("image/gif", MediaKind::Image),
        "mp4" => ("video/mp4", MediaKind::Video),
        "mov" => ("video/mov", MediaKind::Video),
        "webm" => ("video/webm", MediaKind::Video),
        "m4v" | "mkv" => {
            return Err(rejected(format!(
                "Gemini Interactions does not currently accept .{extension} video input"
            )));
        }
        _ => {
            return Err(rejected(format!(
                "Gemini adapter does not support .{extension} media"
            )));
        }
    };

    if kind != asset.media_kind {
        return Err(rejected("asset media kind does not match its source extension"));
    }
    Ok(mime_type)
}

fn analysis_prompt(asset: &Asset) -> String {
    match asset.media_kind {
        MediaKind::Image => {
            "Describe this personal-vlog image factually and concisely. Return a short summary and useful semantic tags. Do not identify unknown people or invent names, relationships, places, events, or intentions. The segments array must be empty.".to_owned()
        }
        MediaKind::Video => {
            let duration = asset
                .technical
                .duration_ms
                .map(|value| format!(" The imported video duration is {value} ms."))
                .unwrap_or_default();
            format!(
                "Analyze this personal-vlog video factually for later story selection. Return a short overall summary, useful semantic tags, and chronological non-overlapping meaningful segments with millisecond start/end offsets. Do not identify unknown people or invent names, relationships, places, events, or intentions.{duration}"
            )
        }
    }
}

fn analysis_schema(kind: MediaKind) -> Value {
    let segment_schema = json!({
        "type": "object",
        "properties": {
            "range": {
                "type": "object",
                "properties": {
                    "start_ms": {"type": "integer", "minimum": 0},
                    "end_ms": {"type": "integer", "minimum": 0}
                },
                "required": ["start_ms", "end_ms"]
            },
            "summary": {"type": "string"},
            "tags": {
                "type": "array",
                "items": {"type": "string"}
            }
        },
        "required": ["range", "summary", "tags"]
    });

    let segments = match kind {
        MediaKind::Image => json!({
            "type": "array",
            "items": segment_schema,
            "maxItems": 0
        }),
        MediaKind::Video => json!({
            "type": "array",
            "items": segment_schema,
            "minItems": 1
        }),
    };

    json!({
        "type": "object",
        "properties": {
            "summary": {"type": "string"},
            "tags": {
                "type": "array",
                "items": {"type": "string"}
            },
            "segments": segments
        },
        "required": ["summary", "tags", "segments"]
    })
}

fn parse_interaction_analysis(
    interaction: &InteractionResponse,
) -> Result<AssetAnalysis, MediaUnderstandingError> {
    let text = interaction
        .steps
        .iter()
        .rev()
        .find(|step| step.kind == "model_output")
        .map(|step| {
            step.content
                .iter()
                .filter(|content| content.kind == "text")
                .filter_map(|content| content.text.as_deref())
                .collect::<String>()
        })
        .filter(|text| !text.trim().is_empty())
        .ok_or_else(|| malformed("Gemini interaction returned no model text output"))?;

    serde_json::from_str(&text)
        .map_err(|error| malformed(format!("Gemini structured output is invalid JSON: {error}")))
}

async fn checked_response(
    response: Response,
    operation: &str,
) -> Result<Response, MediaUnderstandingError> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }

    let body = response.text().await.unwrap_or_default();
    let detail = body.trim().chars().take(512).collect::<String>();
    let message = if detail.is_empty() {
        format!("{operation} failed with HTTP {status}")
    } else {
        format!("{operation} failed with HTTP {status}: {detail}")
    };
    if status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error() {
        Err(unavailable(message))
    } else {
        Err(rejected(message))
    }
}

fn rejected(message: impl Into<String>) -> MediaUnderstandingError {
    MediaUnderstandingError::new(MediaUnderstandingErrorKind::Rejected, message)
}

fn unavailable(message: impl Into<String>) -> MediaUnderstandingError {
    MediaUnderstandingError::new(MediaUnderstandingErrorKind::Unavailable, message)
}

fn malformed(message: impl Into<String>) -> MediaUnderstandingError {
    MediaUnderstandingError::new(MediaUnderstandingErrorKind::MalformedResponse, message)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use laputa_domain::{Asset, AssetId, AssetTechnicalMetadata, MediaKind};
    use serde_json::json;

    use super::{InteractionResponse, analysis_schema, mime_type_for_asset, parse_interaction_analysis};

    fn asset(filename: &str, kind: MediaKind) -> Asset {
        Asset {
            id: AssetId::new(),
            original_path: PathBuf::from(filename),
            display_name: filename.to_owned(),
            media_kind: kind,
            source_sha256: "fixture-revision".to_owned(),
            byte_len: 42,
            technical: AssetTechnicalMetadata {
                duration_ms: (kind == MediaKind::Video).then_some(5_000),
                ..AssetTechnicalMetadata::default()
            },
        }
    }

    #[test]
    fn mime_mapping_covers_supported_interactions_media() {
        assert_eq!(
            mime_type_for_asset(&asset("photo.jpg", MediaKind::Image)).expect("jpeg MIME"),
            "image/jpeg"
        );
        assert_eq!(
            mime_type_for_asset(&asset("photo.heic", MediaKind::Image)).expect("heic MIME"),
            "image/heic"
        );
        assert_eq!(
            mime_type_for_asset(&asset("clip.mp4", MediaKind::Video)).expect("mp4 MIME"),
            "video/mp4"
        );
        assert_eq!(
            mime_type_for_asset(&asset("clip.webm", MediaKind::Video)).expect("webm MIME"),
            "video/webm"
        );
    }

    #[test]
    fn unsupported_story_a_video_container_is_explicitly_rejected() {
        let error = mime_type_for_asset(&asset("clip.mkv", MediaKind::Video))
            .expect_err("mkv should be rejected by this adapter");
        assert!(error.to_string().contains(".mkv"));
    }

    #[test]
    fn image_schema_forbids_temporal_segments() {
        let schema = analysis_schema(MediaKind::Image);
        assert_eq!(schema["properties"]["segments"]["maxItems"], 0);
    }

    #[test]
    fn video_schema_requires_temporal_segments() {
        let schema = analysis_schema(MediaKind::Video);
        assert_eq!(schema["properties"]["segments"]["minItems"], 1);
    }

    #[test]
    fn parses_structured_text_from_rest_interaction_steps() {
        let response: InteractionResponse = serde_json::from_value(json!({
            "status": "completed",
            "steps": [{
                "type": "model_output",
                "content": [{
                    "type": "text",
                    "text": "{\"summary\":\"Kyoto street\",\"tags\":[\"street\"],\"segments\":[]}"
                }]
            }]
        }))
        .expect("fixture response should deserialize");

        let analysis = parse_interaction_analysis(&response).expect("analysis should parse");
        assert_eq!(analysis.summary, "Kyoto street");
        assert_eq!(analysis.tags, vec!["street"]);
        assert!(analysis.segments.is_empty());
    }
}
