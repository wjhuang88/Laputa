use std::sync::Arc;

use anyhow::{Context, Result, ensure};
use async_trait::async_trait;
use laputa_domain::{
    AssetId, DerivedArtifact, GenerationOperation, GenerationRequest, GenerationSource, MediaKind,
};
use laputa_media::ProjectStore;
use laputa_provider::VideoGenerationProvider;
use serde::Deserialize;
use serde_json::{Value, json};
use talos_runtime::{AgentTool, ToolNature, ToolResult};

use crate::generation_orchestration::{
    FfprobeGenerationOutputValidator, GenerationOrchestrator, LocalFileGenerationOutputMaterializer,
};

pub const MEDIA_ANIMATE_PHOTO_TOOL: &str = "media.animate_photo";
pub const MEDIA_GENERATE_BROLL_TOOL: &str = "media.generate_broll";

#[derive(Clone)]
pub struct AnimatePhotoTool<P> {
    project_store: ProjectStore,
    orchestrator: GenerationOrchestrator,
    provider: Arc<P>,
}

impl<P> AnimatePhotoTool<P> {
    #[must_use]
    pub fn new(project_store: ProjectStore, provider: Arc<P>) -> Self {
        let orchestrator = GenerationOrchestrator::at(project_store.root());
        Self {
            project_store,
            orchestrator,
            provider,
        }
    }
}

#[derive(Debug, Deserialize)]
struct AnimatePhotoInput {
    asset_id: AssetId,
    instruction: String,
}

#[async_trait]
impl<P> AgentTool for AnimatePhotoTool<P>
where
    P: VideoGenerationProvider + 'static,
{
    fn name(&self) -> &str {
        MEDIA_ANIMATE_PHOTO_TOOL
    }

    fn description(&self) -> &str {
        "Create a verified derived video artifact that adds motion to one imported photo"
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "asset_id": {"type": "string", "format": "uuid"},
                "instruction": {"type": "string", "minLength": 1}
            },
            "required": ["asset_id", "instruction"],
            "additionalProperties": false
        })
    }

    async fn execute(&self, input: Value) -> ToolResult {
        match self.animate(input).await {
            Ok(artifact) => published_artifact_result(&artifact),
            Err(error) => ToolResult::error(error.to_string()),
        }
    }

    fn nature(&self) -> ToolNature {
        ToolNature::Write
    }
}

impl<P> AnimatePhotoTool<P>
where
    P: VideoGenerationProvider,
{
    async fn animate(&self, input: Value) -> Result<DerivedArtifact> {
        let input: AnimatePhotoInput = serde_json::from_value(input)
            .map_err(|error| anyhow::anyhow!("invalid animate-photo request: {error}"))?;
        ensure!(
            !input.instruction.trim().is_empty(),
            "animate-photo instruction must not be empty"
        );

        let project = self.project_store.open()?;
        let asset = project
            .asset(input.asset_id)
            .with_context(|| format!("asset {} does not exist in the project", input.asset_id))?;
        ensure!(
            asset.media_kind == MediaKind::Image,
            "media.animate_photo requires an image asset"
        );
        let request = GenerationRequest::new(
            GenerationOperation::AnimatePhoto,
            vec![GenerationSource::Asset {
                asset_id: asset.id,
                source_sha256: asset.source_sha256.clone(),
            }],
        );

        self.orchestrator
            .run_video_generation(
                &project,
                request,
                input.instruction,
                self.provider.as_ref(),
                &LocalFileGenerationOutputMaterializer,
                &FfprobeGenerationOutputValidator,
            )
            .await
            .context("media.animate_photo generation failed")
    }
}

#[derive(Clone)]
pub struct GenerateBrollTool<P> {
    project_store: ProjectStore,
    orchestrator: GenerationOrchestrator,
    provider: Arc<P>,
}

impl<P> GenerateBrollTool<P> {
    #[must_use]
    pub fn new(project_store: ProjectStore, provider: Arc<P>) -> Self {
        let orchestrator = GenerationOrchestrator::at(project_store.root());
        Self {
            project_store,
            orchestrator,
            provider,
        }
    }
}

#[derive(Debug, Deserialize)]
struct GenerateBrollInput {
    instruction: String,
}

#[async_trait]
impl<P> AgentTool for GenerateBrollTool<P>
where
    P: VideoGenerationProvider + 'static,
{
    fn name(&self) -> &str {
        MEDIA_GENERATE_BROLL_TOOL
    }

    fn description(&self) -> &str {
        "Create a verified derived B-roll video artifact from a product-level visual instruction"
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "instruction": {"type": "string", "minLength": 1}
            },
            "required": ["instruction"],
            "additionalProperties": false
        })
    }

    async fn execute(&self, input: Value) -> ToolResult {
        match self.generate(input).await {
            Ok(artifact) => published_artifact_result(&artifact),
            Err(error) => ToolResult::error(error.to_string()),
        }
    }

    fn nature(&self) -> ToolNature {
        ToolNature::Write
    }
}

impl<P> GenerateBrollTool<P>
where
    P: VideoGenerationProvider,
{
    async fn generate(&self, input: Value) -> Result<DerivedArtifact> {
        let input: GenerateBrollInput = serde_json::from_value(input)
            .map_err(|error| anyhow::anyhow!("invalid B-roll request: {error}"))?;
        ensure!(
            !input.instruction.trim().is_empty(),
            "B-roll instruction must not be empty"
        );
        let project = self.project_store.open()?;
        let request = GenerationRequest::new(GenerationOperation::GenerateBroll, Vec::new());

        self.orchestrator
            .run_video_generation(
                &project,
                request,
                input.instruction,
                self.provider.as_ref(),
                &LocalFileGenerationOutputMaterializer,
                &FfprobeGenerationOutputValidator,
            )
            .await
            .context("media.generate_broll generation failed")
    }
}

fn published_artifact_result(artifact: &DerivedArtifact) -> ToolResult {
    ToolResult::success(
        json!({
            "status": "published",
            "artifact_id": artifact.id().to_string(),
            "job_id": artifact.job_id().to_string(),
            "request_id": artifact.request_id().to_string(),
            "media_kind": artifact.media_kind(),
            "materialized_path": artifact.materialized_path().to_string_lossy(),
            "sha256": artifact.sha256(),
            "byte_len": artifact.byte_len()
        })
        .to_string(),
    )
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path, process::Command, sync::Arc};

    use anyhow::{Context, Result, ensure};
    use laputa_domain::{AssetTechnicalMetadata, GenerationOperation, GenerationSource, MediaKind};
    use laputa_media::{GenerationStore, MediaProbe, ProjectStore, sha256_file};
    use laputa_provider::{
        FakeGenerationScenario, FakeGenerationStep, FakeVideoGenerationProvider,
    };
    use talos_runtime::AgentTool;

    use super::{AnimatePhotoTool, GenerateBrollTool};

    #[derive(Debug)]
    struct FixtureProbe;

    impl MediaProbe for FixtureProbe {
        fn probe(&self, _source: &Path, kind: MediaKind) -> Result<AssetTechnicalMetadata> {
            Ok(match kind {
                MediaKind::Image => AssetTechnicalMetadata {
                    width: Some(1920),
                    height: Some(1080),
                    ..AssetTechnicalMetadata::default()
                },
                MediaKind::Video => AssetTechnicalMetadata {
                    width: Some(1920),
                    height: Some(1080),
                    duration_ms: Some(2_000),
                    video_codec: Some("fixture".to_owned()),
                    has_audio: false,
                },
            })
        }
    }

    fn prepared_project() -> Result<(tempfile::TempDir, ProjectStore)> {
        let temp = tempfile::tempdir()?;
        let image = temp.path().join("photo.jpg");
        fs::write(&image, b"immutable-original-photo")?;
        let store = ProjectStore::at(temp.path().join("project"));
        store.create("Trip")?;
        store.import_media(&[image], &FixtureProbe)?;
        Ok((temp, store))
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

    fn completed_provider(output: &Path) -> Arc<FakeVideoGenerationProvider> {
        Arc::new(FakeVideoGenerationProvider::new(
            FakeGenerationScenario::Scripted(vec![FakeGenerationStep::Completed {
                locator: output.display().to_string(),
            }]),
        ))
    }

    #[test]
    fn public_generation_tool_schemas_do_not_expose_provider_controls() -> Result<()> {
        let (_temp, store) = prepared_project()?;
        let provider = completed_provider(Path::new("unused.mp4"));
        let schemas = [
            AnimatePhotoTool::new(store.clone(), provider.clone()).parameters(),
            GenerateBrollTool::new(store, provider).parameters(),
        ];

        for schema in schemas {
            let schema = schema.to_string().to_ascii_lowercase();
            for forbidden in ["provider", "vendor", "model", "workflow", "sampler"] {
                assert!(
                    !schema.contains(forbidden),
                    "public Tool schema leaked forbidden control {forbidden}: {schema}"
                );
            }
        }
        Ok(())
    }

    #[tokio::test]
    async fn animate_photo_tool_publishes_verified_artifact_without_mutating_original() -> Result<()>
    {
        let (temp, store) = prepared_project()?;
        let project = store.open()?;
        let asset = project.assets[0].clone();
        let original_hash = sha256_file(&asset.original_path)?;
        let provider_output = temp.path().join("animate-output.mp4");
        write_video_fixture(&provider_output)?;
        let tool = AnimatePhotoTool::new(store.clone(), completed_provider(&provider_output));

        let result = tool
            .execute(serde_json::json!({
                "asset_id": asset.id,
                "instruction": "Add a subtle cinematic push-in while preserving the subject"
            }))
            .await;

        assert!(!result.is_error, "{}", result.content);
        assert!(!result.content.to_ascii_lowercase().contains("provider"));
        let value: serde_json::Value = serde_json::from_str(&result.content)?;
        assert_eq!(value["status"], "published");
        assert_eq!(sha256_file(&asset.original_path)?, original_hash);

        let state = GenerationStore::at(store.root())
            .load(&project)?
            .expect("generation state should exist");
        assert_eq!(state.jobs.len(), 1);
        assert_eq!(state.artifacts.len(), 1);
        assert_eq!(
            state.jobs[0].request().operation,
            GenerationOperation::AnimatePhoto
        );
        assert!(matches!(
            &state.jobs[0].request().sources[..],
            [GenerationSource::Asset { asset_id, source_sha256 }]
                if *asset_id == asset.id && source_sha256 == &asset.source_sha256
        ));
        Ok(())
    }

    #[tokio::test]
    async fn generate_broll_tool_publishes_verified_text_driven_artifact() -> Result<()> {
        let (temp, store) = prepared_project()?;
        let project = store.open()?;
        let provider_output = temp.path().join("broll-output.mp4");
        write_video_fixture(&provider_output)?;
        let tool = GenerateBrollTool::new(store.clone(), completed_provider(&provider_output));

        let result = tool
            .execute(serde_json::json!({
                "instruction": "Create a quiet establishing shot of lanterns at dusk"
            }))
            .await;

        assert!(!result.is_error, "{}", result.content);
        assert!(!result.content.to_ascii_lowercase().contains("provider"));
        let state = GenerationStore::at(store.root())
            .load(&project)?
            .expect("generation state should exist");
        assert_eq!(state.jobs.len(), 1);
        assert_eq!(state.artifacts.len(), 1);
        assert_eq!(
            state.jobs[0].request().operation,
            GenerationOperation::GenerateBroll
        );
        assert!(state.jobs[0].request().sources.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn animate_photo_rejects_non_image_asset_without_creating_generation_state() -> Result<()>
    {
        let temp = tempfile::tempdir()?;
        let video = temp.path().join("clip.mp4");
        fs::write(&video, b"fixture-video")?;
        let store = ProjectStore::at(temp.path().join("project"));
        store.create("Trip")?;
        let report = store.import_media(&[video], &FixtureProbe)?;
        let provider = completed_provider(Path::new("must-not-submit.mp4"));
        let tool = AnimatePhotoTool::new(store.clone(), provider);

        let result = tool
            .execute(serde_json::json!({
                "asset_id": report.imported[0].id,
                "instruction": "Animate this"
            }))
            .await;

        assert!(result.is_error);
        let project = store.open()?;
        assert!(GenerationStore::at(store.root()).load(&project)?.is_none());
        Ok(())
    }
}
