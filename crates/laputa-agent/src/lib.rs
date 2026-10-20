//! Talos runtime composition and Laputa Agent tools.

pub mod first_cut;

use anyhow::{Context, Result, anyhow, bail};
use laputa_domain::{AssetAnalysis, AssetAnalysisState, AssetId};
use laputa_media::{ProjectStore, sha256_file};
use laputa_provider::MediaUnderstandingProvider;

pub use talos_runtime as runtime;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InspectionSource {
    Cache,
    Provider,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InspectionResult {
    pub analysis: AssetAnalysis,
    pub source: InspectionSource,
}

pub async fn inspect_asset(
    store: &ProjectStore,
    provider: &dyn MediaUnderstandingProvider,
    asset_id: AssetId,
) -> Result<InspectionResult> {
    let project = store.open()?;
    let asset = project
        .asset(asset_id)
        .cloned()
        .with_context(|| format!("asset {asset_id} does not exist in the project"))?;
    let source_path = asset.original_path.clone();
    let current_source_sha256 = tokio::task::spawn_blocking(move || sha256_file(&source_path))
        .await
        .context("source revision hashing task failed")?
        .context("failed to verify imported media source revision")?;
    if current_source_sha256 != asset.source_sha256 {
        bail!(
            "asset source revision changed since import; expected {}, current {}",
            asset.source_sha256,
            current_source_sha256
        );
    }

    if let Some(AssetAnalysisState::Ready(record)) = store.load_understanding_state(&asset)? {
        return Ok(InspectionResult {
            analysis: record.analysis,
            source: InspectionSource::Cache,
        });
    }

    let analysis = match provider.analyze(&asset).await {
        Ok(analysis) => analysis,
        Err(error) => {
            let message = format!("media understanding provider failed: {error}");
            store.record_understanding_failure(&asset, &asset.source_sha256, message.clone())?;
            return Err(anyhow!(message));
        }
    };

    if let Err(error) = analysis.validate_for(&asset) {
        let message = format!("media understanding provider returned invalid analysis: {error}");
        store.record_understanding_failure(&asset, &asset.source_sha256, message.clone())?;
        bail!(message);
    }

    store.record_understanding_ready(&asset, &asset.source_sha256, analysis.clone())?;
    Ok(InspectionResult {
        analysis,
        source: InspectionSource::Provider,
    })
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::Path,
        sync::atomic::{AtomicUsize, Ordering},
    };

    use anyhow::Result;
    use async_trait::async_trait;
    use laputa_domain::{
        AnalysisSegment, AnalysisTimeRange, Asset, AssetAnalysis, AssetAnalysisState,
        AssetTechnicalMetadata, MediaKind,
    };
    use laputa_media::{MediaProbe, ProjectStore};
    use laputa_provider::{
        MediaUnderstandingError, MediaUnderstandingErrorKind, MediaUnderstandingProvider,
    };
    use tempfile::TempDir;

    use super::{InspectionSource, inspect_asset};

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
                    duration_ms: Some(2_500),
                    video_codec: Some("fixture".to_owned()),
                    has_audio: true,
                },
            })
        }
    }

    struct FakeProvider {
        calls: AtomicUsize,
        result: Result<AssetAnalysis, MediaUnderstandingError>,
    }

    impl FakeProvider {
        fn ready(analysis: AssetAnalysis) -> Self {
            Self {
                calls: AtomicUsize::new(0),
                result: Ok(analysis),
            }
        }

        fn failing(message: &str) -> Self {
            Self {
                calls: AtomicUsize::new(0),
                result: Err(MediaUnderstandingError::new(
                    MediaUnderstandingErrorKind::Unavailable,
                    message,
                )),
            }
        }

        fn calls(&self) -> usize {
            self.calls.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl MediaUnderstandingProvider for FakeProvider {
        async fn analyze(&self, _asset: &Asset) -> Result<AssetAnalysis, MediaUnderstandingError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.result.clone()
        }
    }

    fn imported_asset(temp: &TempDir, filename: &str) -> Result<(ProjectStore, Asset)> {
        let source = temp.path().join(filename);
        fs::write(&source, b"fixture-media")?;
        let store = ProjectStore::at(temp.path().join("project"));
        store.create("Trip")?;
        let report = store.import_media(&[source], &FixtureProbe)?;
        Ok((store, report.imported[0].clone()))
    }

    fn image_analysis() -> AssetAnalysis {
        AssetAnalysis {
            summary: "A traveler standing near a temple".to_owned(),
            tags: vec!["traveler".to_owned(), "temple".to_owned()],
            segments: Vec::new(),
        }
    }

    fn video_analysis(end_ms: u64) -> AssetAnalysis {
        AssetAnalysis {
            summary: "Walking through a Kyoto street".to_owned(),
            tags: vec!["street".to_owned(), "walking".to_owned()],
            segments: vec![AnalysisSegment {
                range: AnalysisTimeRange {
                    start_ms: 0,
                    end_ms,
                },
                summary: "Traveler walks past storefronts".to_owned(),
                tags: vec!["walking".to_owned(), "storefront".to_owned()],
            }],
        }
    }

    #[tokio::test]
    async fn cached_ready_analysis_skips_second_provider_call() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let (store, asset) = imported_asset(&temp, "photo.jpg")?;
        let provider = FakeProvider::ready(image_analysis());

        let first = inspect_asset(&store, &provider, asset.id).await?;
        let second = inspect_asset(&store, &provider, asset.id).await?;

        assert_eq!(first.source, InspectionSource::Provider);
        assert_eq!(second.source, InspectionSource::Cache);
        assert_eq!(first.analysis, second.analysis);
        assert_eq!(provider.calls(), 1);
        Ok(())
    }

    #[tokio::test]
    async fn valid_video_analysis_is_persisted_as_ready() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let (store, asset) = imported_asset(&temp, "clip.mp4")?;
        let provider = FakeProvider::ready(video_analysis(2_000));

        let result = inspect_asset(&store, &provider, asset.id).await?;
        let state = store.load_understanding_state(&asset)?;

        assert_eq!(result.source, InspectionSource::Provider);
        assert!(matches!(state, Some(AssetAnalysisState::Ready(_))));
        Ok(())
    }

    #[tokio::test]
    async fn invalid_provider_analysis_is_persisted_as_failure_not_ready() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let (store, asset) = imported_asset(&temp, "clip.mp4")?;
        let provider = FakeProvider::ready(video_analysis(3_000));

        let result = inspect_asset(&store, &provider, asset.id).await;
        let state = store.load_understanding_state(&asset)?;

        assert!(result.is_err());
        assert!(matches!(state, Some(AssetAnalysisState::Failed(_))));
        Ok(())
    }

    #[tokio::test]
    async fn provider_failure_is_explicit_and_retryable() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let (store, asset) = imported_asset(&temp, "photo.jpg")?;
        let provider = FakeProvider::failing("fixture provider unavailable");

        let result = inspect_asset(&store, &provider, asset.id).await;
        let state = store.load_understanding_state(&asset)?;

        assert!(result.is_err());
        assert!(matches!(state, Some(AssetAnalysisState::Failed(_))));

        let retry_provider = FakeProvider::ready(image_analysis());
        let retry = inspect_asset(&store, &retry_provider, asset.id).await?;
        assert_eq!(retry.source, InspectionSource::Provider);
        assert_eq!(retry_provider.calls(), 1);
        Ok(())
    }

    #[test]
    fn cache_rejects_stale_expected_source_revision() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let (store, asset) = imported_asset(&temp, "photo.jpg")?;

        let result = store.record_understanding_ready(&asset, "stale-revision", image_analysis());

        assert!(result.is_err());
        assert!(!store.understanding_state_path(asset.id).exists());
        Ok(())
    }

    #[tokio::test]
    async fn externally_changed_source_is_rejected_before_provider_or_cache() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let (store, asset) = imported_asset(&temp, "photo.jpg")?;
        let provider = FakeProvider::ready(image_analysis());
        fs::write(&asset.original_path, b"externally-changed-media")?;

        let result = inspect_asset(&store, &provider, asset.id).await;

        assert!(result.is_err());
        assert_eq!(provider.calls(), 0);
        assert!(!store.understanding_state_path(asset.id).exists());
        Ok(())
    }
}
