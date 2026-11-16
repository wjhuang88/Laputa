//! Rebuildable, project-local visual thumbnails for the real Film Workspace.
//!
//! Originals are always input-only. The cache is a derivative of the imported
//! Asset identity/hash and is never a source of truth for FilmState.
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
};

use anyhow::{Context, Result, ensure};
use gpui::RenderImage;
use image::{Frame, GenericImageView};
use laputa_domain::Asset;
use laputa_media::ProjectStore;

const THUMBNAIL_VERSION: &str = "thumbnail-v1";
const MAX_THUMBNAIL_SIDE: u32 = 384;

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
    if path.exists() {
        if let Ok(image) = decode_cached(&path) {
            return Ok(image);
        }
        // This is a disposable derived file, never the imported original.
        fs::remove_file(&path)
            .with_context(|| format!("failed to discard corrupt thumbnail {}", path.display()))?;
    }
    generate_thumbnail(asset, &path)?;
    decode_cached(&path)
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

fn generate_thumbnail(asset: &Asset, destination: &Path) -> Result<()> {
    let parent = destination
        .parent()
        .context("thumbnail destination has no parent directory")?;
    fs::create_dir_all(parent)
        .with_context(|| format!("cannot create thumbnail cache {}", parent.display()))?;

    // Named .png is explicit so FFmpeg never guesses a muxer from the temporary suffix.
    let temporary = parent.join(format!(
        ".thumbnail-{}-{}.png",
        asset.id,
        std::process::id()
    ));
    let result = (|| {
        let mut command = Command::new("ffmpeg");
        command.args(["-hide_banner", "-loglevel", "error", "-nostdin", "-y"]);
        // Video decodes only its representative first frame; images use the
        // same bounded resize path. Neither code path opens the source for writing.
        command.arg("-i").arg(&asset.original_path);
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

    use super::{cache_path, load_thumbnail};

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
