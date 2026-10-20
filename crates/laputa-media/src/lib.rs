//! Deterministic project storage, media probing, and non-destructive import.

use std::{
    collections::HashSet,
    fs::{self, File},
    io::{BufReader, Read},
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, bail, ensure};
use laputa_domain::{
    Asset, AssetAnalysis, AssetAnalysisFailure, AssetAnalysisRecord, AssetAnalysisState, AssetId,
    AssetTechnicalMetadata, MediaKind, PROJECT_SCHEMA_VERSION, ProjectManifest,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};

const STATE_DIRECTORY: &str = ".laputa";
const MANIFEST_FILE: &str = "project.json";
const UNDERSTANDING_CACHE_FILE: &str = "understanding.json";

pub trait MediaProbe: Send + Sync {
    fn probe(&self, source: &Path, kind: MediaKind) -> Result<AssetTechnicalMetadata>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct FfprobeMediaProbe;

impl MediaProbe for FfprobeMediaProbe {
    fn probe(&self, source: &Path, _kind: MediaKind) -> Result<AssetTechnicalMetadata> {
        let output = Command::new("ffprobe")
            .args([
                "-v",
                "error",
                "-show_entries",
                "stream=codec_type,codec_name,width,height:format=duration",
                "-of",
                "json",
            ])
            .arg(source)
            .output()
            .with_context(
                || "failed to start ffprobe; install FFmpeg or configure another MediaProbe",
            )?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("ffprobe rejected {}: {}", source.display(), stderr.trim());
        }

        let response: FfprobeResponse = serde_json::from_slice(&output.stdout)
            .with_context(|| format!("ffprobe returned invalid JSON for {}", source.display()))?;

        let video = response
            .streams
            .iter()
            .find(|stream| stream.codec_type.as_deref() == Some("video"));
        let has_audio = response
            .streams
            .iter()
            .any(|stream| stream.codec_type.as_deref() == Some("audio"));
        let duration_ms = response
            .format
            .and_then(|format| format.duration)
            .and_then(|value| value.parse::<f64>().ok())
            .filter(|value| value.is_finite() && *value >= 0.0)
            .map(|seconds| (seconds * 1_000.0).round() as u64);

        Ok(AssetTechnicalMetadata {
            width: video.and_then(|stream| stream.width),
            height: video.and_then(|stream| stream.height),
            duration_ms,
            video_codec: video.and_then(|stream| stream.codec_name.clone()),
            has_audio,
        })
    }
}

#[derive(Debug, Deserialize)]
struct FfprobeResponse {
    #[serde(default)]
    streams: Vec<FfprobeStream>,
    format: Option<FfprobeFormat>,
}

#[derive(Debug, Deserialize)]
struct FfprobeStream {
    codec_type: Option<String>,
    codec_name: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct FfprobeFormat {
    duration: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ProjectStore {
    root: PathBuf,
}

impl ProjectStore {
    #[must_use]
    pub fn at(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    #[must_use]
    pub fn state_dir(&self) -> PathBuf {
        self.root.join(STATE_DIRECTORY)
    }

    #[must_use]
    pub fn manifest_path(&self) -> PathBuf {
        self.state_dir().join(MANIFEST_FILE)
    }

    #[must_use]
    pub fn cache_dir(&self) -> PathBuf {
        self.state_dir().join("cache")
    }

    #[must_use]
    pub fn derivatives_dir(&self) -> PathBuf {
        self.state_dir().join("derivatives")
    }

    #[must_use]
    pub fn asset_cache_dir(&self, asset_id: AssetId) -> PathBuf {
        self.cache_dir().join(asset_id.to_string())
    }

    #[must_use]
    pub fn asset_derivatives_dir(&self, asset_id: AssetId) -> PathBuf {
        self.derivatives_dir().join(asset_id.to_string())
    }

    #[must_use]
    pub fn understanding_state_path(&self, asset_id: AssetId) -> PathBuf {
        self.asset_cache_dir(asset_id).join(UNDERSTANDING_CACHE_FILE)
    }

    pub fn create(&self, name: &str) -> Result<ProjectManifest> {
        ensure!(!name.trim().is_empty(), "project name must not be empty");
        ensure!(
            !self.manifest_path().exists(),
            "project already exists at {}",
            self.root.display()
        );

        fs::create_dir_all(self.cache_dir()).with_context(|| {
            format!("failed to create project cache at {}", self.root.display())
        })?;
        fs::create_dir_all(self.derivatives_dir()).with_context(|| {
            format!(
                "failed to create project derivatives at {}",
                self.root.display()
            )
        })?;

        let manifest = ProjectManifest::new(name.trim());
        self.save(&manifest)?;
        Ok(manifest)
    }

    pub fn open(&self) -> Result<ProjectManifest> {
        let bytes = fs::read(self.manifest_path()).with_context(|| {
            format!(
                "failed to read Laputa project manifest at {}",
                self.manifest_path().display()
            )
        })?;
        let manifest: ProjectManifest = serde_json::from_slice(&bytes).with_context(|| {
            format!(
                "invalid Laputa project manifest at {}",
                self.manifest_path().display()
            )
        })?;
        ensure!(
            manifest.schema_version == PROJECT_SCHEMA_VERSION,
            "unsupported project schema {}; expected {}",
            manifest.schema_version,
            PROJECT_SCHEMA_VERSION
        );
        Ok(manifest)
    }

    pub fn import_media(
        &self,
        sources: &[PathBuf],
        probe: &dyn MediaProbe,
    ) -> Result<ImportReport> {
        ensure!(!sources.is_empty(), "at least one media file is required");

        let current = self.open()?;
        let mut candidate = current.clone();
        let state_dir = canonicalize_if_exists(&self.state_dir())?;
        let mut known_paths: HashSet<PathBuf> = current
            .assets
            .iter()
            .map(|asset| asset.original_path.clone())
            .collect();
        let mut imported = Vec::with_capacity(sources.len());

        for source in sources {
            let canonical = fs::canonicalize(source)
                .with_context(|| format!("failed to resolve media file {}", source.display()))?;
            ensure!(
                canonical.is_file(),
                "media source is not a file: {}",
                canonical.display()
            );
            ensure!(
                !canonical.starts_with(&state_dir),
                "Laputa project state cannot be imported as source media: {}",
                canonical.display()
            );
            ensure!(
                known_paths.insert(canonical.clone()),
                "media source is already imported: {}",
                canonical.display()
            );

            let kind = media_kind_from_path(&canonical)?;
            let metadata = fs::metadata(&canonical)
                .with_context(|| format!("failed to inspect {}", canonical.display()))?;
            let technical = probe.probe(&canonical, kind)?;
            let source_sha256 = sha256_file(&canonical)?;
            let display_name = canonical
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| canonical.display().to_string());

            let asset = Asset {
                id: AssetId::new(),
                original_path: canonical,
                display_name,
                media_kind: kind,
                source_sha256,
                byte_len: metadata.len(),
                technical,
            };
            candidate.assets.push(asset.clone());
            imported.push(asset);
        }

        for asset in &imported {
            fs::create_dir_all(self.asset_cache_dir(asset.id)).with_context(|| {
                format!("failed to create cache directory for asset {}", asset.id)
            })?;
            fs::create_dir_all(self.asset_derivatives_dir(asset.id)).with_context(|| {
                format!(
                    "failed to create derivative directory for asset {}",
                    asset.id
                )
            })?;
        }
        self.save(&candidate)?;

        Ok(ImportReport {
            project: candidate,
            imported,
        })
    }

    pub fn load_understanding_state(&self, asset: &Asset) -> Result<Option<AssetAnalysisState>> {
        let path = self.understanding_state_path(asset.id);
        if !path.exists() {
            return Ok(None);
        }

        let bytes = fs::read(&path)
            .with_context(|| format!("failed to read understanding cache {}", path.display()))?;
        let state: AssetAnalysisState = serde_json::from_slice(&bytes)
            .with_context(|| format!("invalid understanding cache {}", path.display()))?;
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

    pub fn record_understanding_ready(
        &self,
        asset: &Asset,
        expected_source_sha256: &str,
        analysis: AssetAnalysis,
    ) -> Result<()> {
        self.ensure_understanding_revision(asset, expected_source_sha256)?;
        analysis
            .validate_for(asset)
            .context("cannot persist invalid asset analysis")?;
        self.write_understanding_state(
            asset.id,
            &AssetAnalysisState::Ready(AssetAnalysisRecord {
                source_sha256: expected_source_sha256.to_owned(),
                analysis,
            }),
        )
    }

    pub fn record_understanding_failure(
        &self,
        asset: &Asset,
        expected_source_sha256: &str,
        message: impl Into<String>,
    ) -> Result<()> {
        self.ensure_understanding_revision(asset, expected_source_sha256)?;
        let message = message.into();
        ensure!(
            !message.trim().is_empty(),
            "analysis failure message must not be empty"
        );
        self.write_understanding_state(
            asset.id,
            &AssetAnalysisState::Failed(AssetAnalysisFailure {
                source_sha256: expected_source_sha256.to_owned(),
                message,
            }),
        )
    }

    fn ensure_understanding_revision(
        &self,
        asset: &Asset,
        expected_source_sha256: &str,
    ) -> Result<()> {
        ensure!(
            asset.source_sha256 == expected_source_sha256,
            "asset source revision changed; expected {expected_source_sha256}, current {}",
            asset.source_sha256
        );
        Ok(())
    }

    fn write_understanding_state(
        &self,
        asset_id: AssetId,
        state: &AssetAnalysisState,
    ) -> Result<()> {
        let path = self.understanding_state_path(asset_id);
        let parent = path
            .parent()
            .context("understanding cache path has no parent directory")?;
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create understanding cache directory {}",
                parent.display()
            )
        })?;
        let bytes =
            serde_json::to_vec_pretty(state).context("failed to serialize understanding cache")?;
        let temporary = parent.join(format!(
            ".{UNDERSTANDING_CACHE_FILE}.tmp-{}",
            std::process::id()
        ));
        fs::write(&temporary, bytes).with_context(|| {
            format!(
                "failed to write temporary understanding cache {}",
                temporary.display()
            )
        })?;
        File::open(&temporary)
            .and_then(|file| file.sync_all())
            .with_context(|| {
                format!(
                    "failed to sync temporary understanding cache {}",
                    temporary.display()
                )
            })?;

        match fs::rename(&temporary, &path) {
            Ok(()) => Ok(()),
            Err(error) if path.exists() => {
                fs::remove_file(&path).with_context(|| {
                    format!("failed to replace understanding cache after rename error: {error}")
                })?;
                fs::rename(&temporary, &path)
                    .context("failed to install replacement understanding cache")
            }
            Err(error) => Err(error).context("failed to install understanding cache"),
        }
    }

    fn save(&self, manifest: &ProjectManifest) -> Result<()> {
        fs::create_dir_all(self.state_dir()).with_context(|| {
            format!(
                "failed to create Laputa state directory at {}",
                self.root.display()
            )
        })?;
        let bytes =
            serde_json::to_vec_pretty(manifest).context("failed to serialize project manifest")?;
        let temporary = self
            .state_dir()
            .join(format!(".{MANIFEST_FILE}.tmp-{}", std::process::id()));
        fs::write(&temporary, bytes).with_context(|| {
            format!("failed to write temporary manifest {}", temporary.display())
        })?;
        File::open(&temporary)
            .and_then(|file| file.sync_all())
            .with_context(|| {
                format!("failed to sync temporary manifest {}", temporary.display())
            })?;

        match fs::rename(&temporary, self.manifest_path()) {
            Ok(()) => Ok(()),
            Err(error) if self.manifest_path().exists() => {
                fs::remove_file(self.manifest_path()).with_context(|| {
                    format!("failed to replace manifest after rename error: {error}")
                })?;
                fs::rename(&temporary, self.manifest_path())
                    .context("failed to install replacement project manifest")
            }
            Err(error) => Err(error).context("failed to install project manifest"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ImportReport {
    pub project: ProjectManifest,
    pub imported: Vec<Asset>,
}

pub fn sha256_file(path: &Path) -> Result<String> {
    let file = File::open(path)
        .with_context(|| format!("failed to open {} for hashing", path.display()))?;
    let mut reader = BufReader::new(file);
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];

    loop {
        let read = reader
            .read(&mut buffer)
            .with_context(|| format!("failed while hashing {}", path.display()))?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }

    Ok(format!("{:x}", digest.finalize()))
}

fn media_kind_from_path(path: &Path) -> Result<MediaKind> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .with_context(|| format!("media file has no usable extension: {}", path.display()))?;

    match extension.as_str() {
        "jpg" | "jpeg" | "png" | "webp" | "heic" | "heif" | "gif" => Ok(MediaKind::Image),
        "mp4" | "mov" | "m4v" | "mkv" | "webm" => Ok(MediaKind::Video),
        _ => bail!("unsupported media type .{extension}: {}", path.display()),
    }
}

fn canonicalize_if_exists(path: &Path) -> Result<PathBuf> {
    if path.exists() {
        fs::canonicalize(path).with_context(|| format!("failed to resolve {}", path.display()))
    } else {
        Ok(path.to_path_buf())
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use anyhow::Result;
    use laputa_domain::{AssetTechnicalMetadata, MediaKind};
    use tempfile::tempdir;

    use super::{MediaProbe, ProjectStore, sha256_file};

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

    #[test]
    fn import_preserves_original_bytes_and_reopens_inventory() -> Result<()> {
        let temp = tempdir()?;
        let originals = temp.path().join("originals");
        fs::create_dir_all(&originals)?;
        let image = originals.join("photo.jpg");
        let video = originals.join("clip.mp4");
        fs::write(&image, b"fixture-image-bytes")?;
        fs::write(&video, b"fixture-video-bytes")?;
        let image_before = sha256_file(&image)?;
        let video_before = sha256_file(&video)?;

        let store = ProjectStore::at(temp.path().join("project"));
        store.create("Trip")?;
        let report = store.import_media(&[image.clone(), video.clone()], &FixtureProbe)?;

        assert_eq!(report.imported.len(), 2);
        assert_eq!(report.imported[0].source_sha256, image_before);
        assert_eq!(report.imported[1].source_sha256, video_before);
        assert_eq!(sha256_file(&image)?, image_before);
        assert_eq!(sha256_file(&video)?, video_before);
        for asset in &report.imported {
            assert!(
                store
                    .asset_cache_dir(asset.id)
                    .starts_with(store.state_dir())
            );
            assert!(
                store
                    .asset_derivatives_dir(asset.id)
                    .starts_with(store.state_dir())
            );
            assert_ne!(asset.original_path, store.asset_cache_dir(asset.id));
        }

        let reopened = store.open()?;
        assert_eq!(reopened, report.project);
        assert_eq!(reopened.assets.len(), 2);
        Ok(())
    }

    #[test]
    fn invalid_batch_does_not_partially_mutate_manifest() -> Result<()> {
        let temp = tempdir()?;
        let image = temp.path().join("photo.jpg");
        let unsupported = temp.path().join("notes.txt");
        fs::write(&image, b"image")?;
        fs::write(&unsupported, b"not-media")?;

        let store = ProjectStore::at(temp.path().join("project"));
        let baseline = store.create("Trip")?;
        let result = store.import_media(&[image, unsupported], &FixtureProbe);

        assert!(result.is_err());
        assert_eq!(store.open()?, baseline);
        Ok(())
    }

    #[derive(Debug)]
    struct FailingProbe;

    impl MediaProbe for FailingProbe {
        fn probe(&self, _source: &Path, _kind: MediaKind) -> Result<AssetTechnicalMetadata> {
            anyhow::bail!("fixture probe failure")
        }
    }

    #[test]
    fn probe_failure_does_not_mutate_manifest_or_original() -> Result<()> {
        let temp = tempdir()?;
        let image = temp.path().join("photo.jpg");
        fs::write(&image, b"image")?;
        let before = sha256_file(&image)?;

        let store = ProjectStore::at(temp.path().join("project"));
        let baseline = store.create("Trip")?;
        let result = store.import_media(std::slice::from_ref(&image), &FailingProbe);

        assert!(result.is_err());
        assert_eq!(store.open()?, baseline);
        assert_eq!(sha256_file(&image)?, before);
        Ok(())
    }

    #[test]
    fn duplicate_import_is_rejected_without_manifest_change() -> Result<()> {
        let temp = tempdir()?;
        let image = temp.path().join("photo.jpg");
        fs::write(&image, b"image")?;

        let store = ProjectStore::at(temp.path().join("project"));
        store.create("Trip")?;
        let first = store.import_media(std::slice::from_ref(&image), &FixtureProbe)?;
        let second = store.import_media(std::slice::from_ref(&image), &FixtureProbe);

        assert!(second.is_err());
        assert_eq!(store.open()?, first.project);
        Ok(())
    }
}
