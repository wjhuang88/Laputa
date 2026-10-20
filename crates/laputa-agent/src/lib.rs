//! Talos runtime composition and Laputa Agent tools.

use std::{
    fs::{self, File},
    path::PathBuf,
};

use anyhow::{Context, Result, anyhow, bail, ensure};
use laputa_domain::{
    Asset, AssetAnalysis, AssetAnalysisFailure, AssetAnalysisRecord, AssetAnalysisState, AssetId,
};
use laputa_media::ProjectStore;
use laputa_provider::MediaUnderstandingProvider;

pub use talos_runtime as runtime;

const UNDERSTANDING_CACHE_FILE: &str = "understanding.json";

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

#[derive(Clone, Debug)]
pub struct AssetUnderstandingCache {
    store: ProjectStore,
}

impl AssetUnderstandingCache {
    #[must_use]
    pub fn new(store: ProjectStore) -> Self {
        Self { store }
    }

    #[must_use]
    pub fn state_path(&self, asset_id: AssetId) -> PathBuf {
        self.store
            .asset_cache_dir(asset_id)
            .join(UNDERSTANDING_CACHE_FILE)
    }

    pub fn load_state(&self, asset: &Asset) -> Result<Option<AssetAnalysisState>> {
        let path = self.state_path(asset.id);
        if !path.exists() {
            return Ok(None);
        }

        let bytes = fs::read(&path)
            .with_context(|| format!("failed to read analysis cache {}", path.display()))?;
        let state: AssetAnalysisState = serde_json::from_slice(&bytes)
            .with_context(|| format!("invalid analysis cache {}", path.display()))?;

        let source_sha256 = match &state {
            AssetAnalysisState::Ready(record) => &record.source_sha256,
            AssetAnalysisState::Failed(failure) => &failure.source_sha256,
        };
        if source_sha256 != &asset.source_sha256 {
            return Ok(None);
        }

        if let AssetAnalysisState::Ready(record) = &state {
            record
                .analysis
                .validate_for(asset)
                .context("cached analysis is invalid for the current asset")?;
        }

        Ok(Some(state))
    }

    pub fn record_ready(
        &self,
        asset: &Asset,
        expected_source_sha256: &str,
        analysis: AssetAnalysis,
    ) -> Result<()> {
        self.ensure_revision(asset, expected_source_sha256)?;
        analysis
            .validate_for(asset)
            .context("cannot persist invalid asset analysis")?;
        self.write_state(
            asset.id,
            &AssetAnalysisState::Ready(AssetAnalysisRecord {
                source_sha256: expected_source_sha256.to_owned(),
                analysis,
            }),
        )
    }

    pub fn record_failure(
        &self,
        asset: &Asset,
        expected_source_sha256: &str,
        message: impl Into<String>,
    ) -> Result<()> {
        self.ensure_revision(asset, expected_source_sha256)?;
        let message = message.into();
        ensure!(
            !message.trim().is_empty(),
            "analysis failure message must not be empty"
        );
        self.write_state(
            asset.id,
            &AssetAnalysisState::Failed(AssetAnalysisFailure {
                source_sha256: expected_source_sha256.to_owned(),
                message,
            }),
        )
    }

    fn ensure_revision(&self, asset: &Asset, expected_source_sha256: &str) -> Result<()> {
        ensure!(
            asset.source_sha256 == expected_source_sha256,
            "asset source revision changed; expected {expected_source_sha256}, current {}",
            asset.source_sha256
        );
        Ok(())
    }

    fn write_state(&self, asset_id: AssetId, state: &AssetAnalysisState) -> Result<()> {
        let path = self.state_path(asset_id);
        let parent = path
            .parent()
            .context("analysis cache path has no parent directory")?;
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create analysis cache at {}", parent.display()))?;

        let bytes =
            serde_json::to_vec_pretty(state).context("failed to serialize analysis cache")?;
        let temporary = parent.join(format!(
            ".{UNDERSTANDING_CACHE_FILE}.tmp-{}",
            std::process::id()
        ));
        fs::write(&temporary, bytes).with_context(|| {
            format!(
                "failed to write temporary analysis cache {}",
                temporary.display()
            )
        })?;
        File::open(&temporary)
            .and_then(|file| file.sync_all())
            .with_context(|| {
                format!(
                    "failed to sync temporary analysis cache {}",
                    temporary.display()
                )
            })?;

        match fs::rename(&temporary, &path) {
            Ok(()) => Ok(()),
            Err(error) if path.exists() => {
                fs::remove_file(&path).with_context(|| {
                    format!("failed to replace analysis cache after rename error: {error}")
                })?;
                fs::rename(&temporary, &path)
                    .context("failed to install replacement analysis cache")
            }
            Err(error) => Err(error).context("failed to install analysis cache"),
        }
    }
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
    let cache = AssetUnderstandingCache::new(store.clone());

    if let Some(AssetAnalysisState::Ready(record)) = cache.load_state(&asset)? {
        return Ok(InspectionResult {
            analysis: record.analysis,
            source: InspectionSource::Cache,
        });
    }

    let analysis = match provider.analyze(&asset).await {
        Ok(analysis) => analysis,
        Err(error) => {
            let message = format!("media understanding provider failed: {error}");
            cache.record_failure(&asset, &asset.source_sha256, message.clone())?;
            return Err(anyhow!(message));
        }
    };

    if let Err(error) = analysis.validate_for(&asset) {
        let message = format!("media understanding provider returned invalid analysis: {error}");
        cache.record_failure(&asset, &asset.source_sha256, message.clone())?;
        bail!(message);
    }

    cache.record_ready(&asset, &asset.source_sha256, analysis.clone())?;
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

    use super::{AssetUnderstandingCache, InspectionSource, inspect_asset};

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
        let cache = AssetUnderstandingCache::new(store);
        let state = cache.load_state(&asset)?;

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
        let cache = AssetUnderstandingCache::new(store);
        let state = cache.load_state(&asset)?;

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
        let cache = AssetUnderstandingCache::new(store.clone());
        let state = cache.load_state(&asset)?;

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
        let cache = AssetUnderstandingCache::new(store);

        let result = cache.record_ready(&asset, "stale-revision", image_analysis());

        assert!(result.is_err());
        assert!(!cache.state_path(asset.id).exists());
        Ok(())
    }
}
