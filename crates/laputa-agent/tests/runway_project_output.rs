use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
};

use anyhow::{Context, Result, ensure};
use async_trait::async_trait;
use laputa_agent::generation_tools::GenerateBrollTool;
use laputa_media::{GenerationStore, ProjectStore, sha256_file};
use laputa_provider::{
    FakeGenerationScenario, FakeGenerationStep, FakeVideoGenerationProvider,
    GenerationProviderError, GenerationProviderErrorKind,
};
use laputa_provider_runway::{RunwayOutputFetcher, RunwayProjectVideoGenerationProvider};
use talos_runtime::AgentTool;

struct FixtureRunwayOutputFetcher {
    source: PathBuf,
}

#[async_trait]
impl RunwayOutputFetcher for FixtureRunwayOutputFetcher {
    async fn fetch(
        &self,
        _locator: &str,
        destination: &Path,
    ) -> std::result::Result<(), GenerationProviderError> {
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

#[tokio::test]
async fn runway_project_wrapper_composes_with_tool_and_e3_publication_gate() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let store = ProjectStore::at(temp.path().join("project"));
    store.create("Trip")?;

    let remote_fixture = temp.path().join("runway-remote-output.mp4");
    write_video_fixture(&remote_fixture)?;
    let remote_hash = sha256_file(&remote_fixture)?;

    let inner = FakeVideoGenerationProvider::new(FakeGenerationScenario::Scripted(vec![
        FakeGenerationStep::Completed {
            locator: "https://cdn.example.test/runway-output.mp4".to_owned(),
        },
    ]));
    let provider = RunwayProjectVideoGenerationProvider::with_fetcher(
        inner,
        store.root(),
        FixtureRunwayOutputFetcher {
            source: remote_fixture,
        },
    )?;
    let tool = GenerateBrollTool::new(store.clone(), Arc::new(provider));

    let result = tool
        .execute(serde_json::json!({
            "instruction": "Create a quiet establishing shot of lanterns at dusk"
        }))
        .await;

    ensure!(!result.is_error, "{}", result.content);
    let value: serde_json::Value = serde_json::from_str(&result.content)?;
    ensure!(
        value["status"] == "published",
        "unexpected Tool result: {value}"
    );

    let staging_root = store.root().join(".laputa/provider-staging/runway");
    let mut staged_tasks = fs::read_dir(&staging_root)?;
    let staged_task = staged_tasks
        .next()
        .transpose()?
        .context("Runway project wrapper did not create a staging task directory")?;
    ensure!(
        staged_tasks.next().is_none(),
        "expected exactly one Runway staging task directory"
    );
    let staged_output = staged_task.path().join("output.mp4");
    ensure!(staged_output.is_file(), "Runway staging output is missing");
    ensure!(
        sha256_file(&staged_output)? == remote_hash,
        "Runway staging output changed fixture bytes"
    );

    let project = store.open()?;
    let state = GenerationStore::at(store.root())
        .load(&project)?
        .context("generation state should exist")?;
    ensure!(state.jobs.len() == 1, "expected one generation job");
    ensure!(
        state.artifacts.len() == 1,
        "expected one published artifact"
    );
    let artifact = &state.artifacts[0];
    ensure!(
        artifact
            .materialized_path()
            .starts_with(store.root().join(".laputa/derivatives")),
        "published Artifact must live under project derivatives"
    );
    ensure!(
        artifact.materialized_path() != staged_output,
        "Runway staging output must not itself become the published Artifact"
    );
    ensure!(
        sha256_file(artifact.materialized_path())? == remote_hash,
        "published Artifact bytes differ from verified staged output"
    );
    ensure!(
        artifact.sha256() == remote_hash,
        "published Artifact SHA-256 does not match materialized bytes"
    );

    Ok(())
}
