//! Rebuildable, project-local visual thumbnails for the real Film Workspace.
//!
//! Originals are always input-only. The cache is a derivative of the imported
//! Asset identity/hash and is never a source of truth for FilmState.
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use anyhow::{Context, Result, ensure};
use gpui::RenderImage;
use image::{Frame, GenericImageView};
use laputa_domain::{Asset, MediaKind};
use laputa_media::ProjectStore;

const THUMBNAIL_VERSION: &str = "thumbnail-v1";
const MAX_THUMBNAIL_SIDE: u32 = 384;
static NEXT_TEMPORARY_THUMBNAIL: AtomicU64 = AtomicU64::new(0);

pub fn cache_path(store: &ProjectStore, asset: &Asset) -> Result<PathBuf> {
    ensure!(
        asset.source_sha256.len() == 64
            && asset
                .source_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit()),
        "invalid imported media checksum"
    );
    Ok(store
        .asset_cache_dir(asset.id)
        .join(format!("{THUMBNAIL_VERSION}-{}.png", asset.source_sha256)))
}

/// Called only on GPUI's background executor. Decodes at most a small,
/// bounded, project-owned PNG; a malformed cache is discarded and rebuilt.
pub fn load_thumbnail(store: &ProjectStore, asset: &Asset) -> Result<Arc<RenderImage>> {
    let path = cache_path(store, asset)?;
    load_thumbnail_at_path(asset, &path, None)
}

/// The Scene Strip and selection rail must show the first frame *of the cut*,
/// not the first frame of an imported video. A source-in-point change yields
/// a different derivative key; no original bytes or project state are written.
pub fn scene_video_cache_path(
    store: &ProjectStore,
    asset: &Asset,
    source_start_ms: u64,
) -> Result<PathBuf> {
    ensure!(
        asset.media_kind == MediaKind::Video,
        "a scene video thumbnail requires a video source"
    );
    let _ = cache_path(store, asset)?; // reuse SHA-256 checksum validation
    Ok(store.asset_cache_dir(asset.id).join(format!(
        "scene-thumbnail-v1-{}-{source_start_ms}.png",
        asset.source_sha256
    )))
}

pub fn load_scene_video_thumbnail(
    store: &ProjectStore,
    asset: &Asset,
    source_start_ms: u64,
) -> Result<Arc<RenderImage>> {
    let path = scene_video_cache_path(store, asset, source_start_ms)?;
    load_thumbnail_at_path(asset, &path, Some(source_start_ms))
}

fn load_thumbnail_at_path(
    asset: &Asset,
    path: &Path,
    source_start_ms: Option<u64>,
) -> Result<Arc<RenderImage>> {
    if path.exists() {
        if let Ok(image) = decode_cached(path) {
            return Ok(image);
        }
        // This is a disposable derived file, never the imported original.
        fs::remove_file(path)
            .with_context(|| format!("failed to discard corrupt thumbnail {}", path.display()))?;
    }
    generate_thumbnail(asset, path, source_start_ms)?;
    decode_cached(path)
}

fn decode_cached(path: &Path) -> Result<Arc<RenderImage>> {
    // Protect GPU memory if a corrupt cache is externally replaced by a huge image.
    let bytes = fs::read(path).with_context(|| format!("cannot read {}", path.display()))?;
    let image = image::load_from_memory(&bytes)
        .with_context(|| format!("cannot decode {}", path.display()))?;
    let (width, height) = image.dimensions();
    ensure!(
        width > 0 && height > 0 && width <= MAX_THUMBNAIL_SIDE && height <= MAX_THUMBNAIL_SIDE,
        "cached thumbnail dimensions are out of bounds"
    );
    Ok(Arc::new(RenderImage::new(vec![Frame::new(
        image.into_rgba8(),
    )])))
}

/// A retry can overlap an obsolete background job even after the UI epoch
/// invalidates its completion. Give every attempt a distinct disposable path.
fn temporary_thumbnail_path(asset: &Asset, parent: &Path) -> PathBuf {
    let attempt = NEXT_TEMPORARY_THUMBNAIL.fetch_add(1, Ordering::Relaxed);
    parent.join(format!(
        ".thumbnail-{}-{}-{attempt}.png",
        asset.id,
        std::process::id()
    ))
}

fn generate_thumbnail(
    asset: &Asset,
    destination: &Path,
    source_start_ms: Option<u64>,
) -> Result<()> {
    let parent = destination
        .parent()
        .context("thumbnail destination has no parent directory")?;
    fs::create_dir_all(parent)
        .with_context(|| format!("cannot create thumbnail cache {}", parent.display()))?;

    // Named .png is explicit so FFmpeg never guesses a muxer from the temporary suffix.
    let temporary = temporary_thumbnail_path(asset, parent);
    let result = (|| {
        let mut command = Command::new("ffmpeg");
        command.args(["-hide_banner", "-loglevel", "error", "-nostdin", "-y"]);
        // An output-side seek selects the scene's source in-point precisely.
        // Originals are input-only and any generated image stays in the cache.
        command.arg("-i").arg(&asset.original_path);
        if let Some(start_ms) = source_start_ms {
            command
                .arg("-ss")
                .arg(format!("{}.{:03}", start_ms / 1_000, start_ms % 1_000));
        }
        command.args([
            "-map",
            "0:v:0",
            "-frames:v",
            "1",
            "-vf",
            "scale=384:216:force_original_aspect_ratio=decrease",
            "-c:v",
            "png",
            "-f",
            "image2",
            "-update",
            "1",
        ]);
        let output = command
            .arg(&temporary)
            .output()
            .context("cannot run FFmpeg for a media thumbnail")?;
        ensure!(
            output.status.success(),
            "thumbnail decode failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
        // Preflight the derivative before publishing it as cache.
        decode_cached(&temporary)?;
        fs::rename(&temporary, destination)
            .with_context(|| format!("cannot publish thumbnail {}", destination.display()))?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

#[cfg(test)]
mod tests {
    use std::{fs, process::Command};

    use anyhow::{Context, Result};
    use image::{Rgba, RgbaImage};
    use laputa_domain::{Asset, AssetId, AssetTechnicalMetadata, MediaKind};
    use laputa_media::ProjectStore;

    use super::{
        cache_path, load_scene_video_thumbnail, load_thumbnail, scene_video_cache_path,
        temporary_thumbnail_path,
    };

    fn fixture_asset(source: &std::path::Path, kind: MediaKind) -> Result<Asset> {
        Ok(Asset {
            id: AssetId::new(),
            original_path: source.to_path_buf(),
            display_name: "sample".to_owned(),
            media_kind: kind,
            source_sha256: "a".repeat(64),
            byte_len: fs::metadata(source)?.len(),
            technical: AssetTechnicalMetadata::default(),
        })
    }

    #[test]
    fn real_image_thumbnail_is_rebuildable_and_original_is_immutable() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let original = temp.path().join("original.png");
        RgbaImage::from_pixel(120, 80, Rgba([145, 78, 50, 255])).save(&original)?;
        let bytes_before = fs::read(&original)?;
        let store = ProjectStore::at(temp.path().join("project"));
        let asset = fixture_asset(&original, MediaKind::Image)?;

        let first = load_thumbnail(&store, &asset)?;
        assert_eq!(first.frame_count(), 1);
        let cache = cache_path(&store, &asset)?;
        assert!(cache.is_file());
        assert!(cache.starts_with(store.cache_dir()));
        fs::write(&cache, b"broken cache entry")?;
        let rebuilt = load_thumbnail(&store, &asset)?;
        assert_eq!(rebuilt.frame_count(), 1);
        assert_eq!(fs::read(&original)?, bytes_before);
        Ok(())
    }

    #[test]
    fn real_video_thumbnail_decodes_first_frame_without_mutating_video() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let original = temp.path().join("original.mp4");
        let output = Command::new("ffmpeg")
            .args(["-hide_banner", "-loglevel", "error", "-nostdin", "-y"])
            .args(["-f", "lavfi", "-i", "testsrc=size=320x180:rate=10"])
            .args(["-t", "0.6", "-pix_fmt", "yuv420p", "-c:v", "mpeg4"])
            .arg(&original)
            .output()
            .context("FFmpeg unavailable for thumbnail fixture")?;
        assert!(
            output.status.success(),
            "could not prepare video fixture: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let before = fs::read(&original)?;
        let asset = fixture_asset(&original, MediaKind::Video)?;
        let store = ProjectStore::at(temp.path().join("project"));
        assert_eq!(load_thumbnail(&store, &asset)?.frame_count(), 1);
        assert_eq!(fs::read(original)?, before);
        Ok(())
    }

    #[test]
    fn reused_video_at_two_inpoints_has_distinct_rebuildable_thumbnails() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let original = temp.path().join("moving.mp4");
        let output = Command::new("ffmpeg")
            .args(["-hide_banner", "-loglevel", "error", "-nostdin", "-y"])
            .args(["-f", "lavfi", "-i", "testsrc2=size=320x180:rate=12"])
            .args(["-t", "3", "-pix_fmt", "yuv420p", "-c:v", "mpeg4"])
            .arg(&original)
            .output()
            .context("FFmpeg unavailable for moving video thumbnail fixture")?;
        assert!(
            output.status.success(),
            "could not generate moving-video fixture: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let original_before = fs::read(&original)?;
        let asset = fixture_asset(&original, MediaKind::Video)?;
        let store = ProjectStore::at(temp.path().join("project"));
        let first_path = scene_video_cache_path(&store, &asset, 0)?;
        let later_path = scene_video_cache_path(&store, &asset, 1_500)?;
        assert_ne!(first_path, later_path);
        assert_eq!(
            load_scene_video_thumbnail(&store, &asset, 0)?.frame_count(),
            1
        );
        assert_eq!(
            load_scene_video_thumbnail(&store, &asset, 1_500)?.frame_count(),
            1
        );
        assert_ne!(
            fs::read(&first_path)?,
            fs::read(&later_path)?,
            "two different video offsets must not show the same frame"
        );
        fs::write(&later_path, b"corrupted derivative")?;
        assert_eq!(
            load_scene_video_thumbnail(&store, &asset, 1_500)?.frame_count(),
            1
        );
        assert_ne!(fs::read(later_path)?, b"corrupted derivative");
        let beyond_path = scene_video_cache_path(&store, &asset, 8_000)?;
        assert!(load_scene_video_thumbnail(&store, &asset, 8_000).is_err());
        assert!(!beyond_path.exists());
        assert_eq!(fs::read(&original)?, original_before);
        Ok(())
    }

    #[test]
    fn overlapping_thumbnail_retry_attempts_never_share_a_temporary_file() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let asset = Asset {
            id: AssetId::new(),
            original_path: temp.path().join("source.jpg"),
            display_name: "source".to_owned(),
            media_kind: MediaKind::Image,
            source_sha256: "a".repeat(64),
            byte_len: 0,
            technical: AssetTechnicalMetadata::default(),
        };
        let one = temporary_thumbnail_path(&asset, temp.path());
        let two = temporary_thumbnail_path(&asset, temp.path());
        assert_ne!(one, two);
        assert_eq!(one.parent(), Some(temp.path()));
        assert_eq!(two.parent(), Some(temp.path()));
        assert!(!one.exists() && !two.exists());
        Ok(())
    }

    #[test]
    fn missing_source_fails_without_publishing_a_cache_file() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let original = temp.path().join("missing.mp4");
        let asset = Asset {
            id: AssetId::new(),
            original_path: original,
            display_name: "missing".to_owned(),
            media_kind: MediaKind::Video,
            source_sha256: "b".repeat(64),
            byte_len: 0,
            technical: AssetTechnicalMetadata::default(),
        };
        let store = ProjectStore::at(temp.path().join("project"));
        assert!(load_thumbnail(&store, &asset).is_err());
        assert!(!cache_path(&store, &asset)?.exists());
        Ok(())
    }
}
