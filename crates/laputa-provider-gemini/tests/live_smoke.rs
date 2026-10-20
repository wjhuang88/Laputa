//! Opt-in live smoke for the Gemini media-understanding adapter.
//!
//! This test is ignored by default because it uploads real media and consumes an
//! external API. Run it only with explicit credentials and a local fixture path.

use std::{env, fs, path::PathBuf};

use laputa_domain::{Asset, AssetId, AssetTechnicalMetadata, MediaKind};
use laputa_provider::MediaUnderstandingProvider;
use laputa_provider_gemini::{GeminiConfig, GeminiMediaUnderstandingProvider};

#[tokio::test]
#[ignore = "requires GEMINI_API_KEY, LAPUTA_GEMINI_MODEL and LAPUTA_GEMINI_SMOKE_MEDIA"]
async fn live_media_understanding_smoke() {
    let api_key = env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY must be set");
    let model = env::var("LAPUTA_GEMINI_MODEL").expect("LAPUTA_GEMINI_MODEL must be set");
    let source = PathBuf::from(
        env::var("LAPUTA_GEMINI_SMOKE_MEDIA")
            .expect("LAPUTA_GEMINI_SMOKE_MEDIA must point to a local image or video"),
    );
    let metadata = fs::metadata(&source).expect("live smoke media must exist");
    assert!(
        metadata.is_file(),
        "live smoke media must be a regular file"
    );

    let media_kind = media_kind_for_smoke(&source);
    let display_name = source
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| source.display().to_string());
    let asset = Asset {
        id: AssetId::new(),
        original_path: source,
        display_name,
        media_kind,
        source_sha256: "live-smoke-revision".to_owned(),
        byte_len: metadata.len(),
        technical: AssetTechnicalMetadata::default(),
    };

    let config = GeminiConfig::new(api_key, model).expect("live smoke config should be valid");
    let provider = GeminiMediaUnderstandingProvider::new(config)
        .expect("live smoke provider should initialize");
    let analysis = provider
        .analyze(&asset)
        .await
        .expect("Gemini live media understanding should succeed");

    analysis
        .validate_for(&asset)
        .expect("Gemini result must satisfy the canonical Laputa analysis contract");
}

fn media_kind_for_smoke(path: &std::path::Path) -> MediaKind {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .expect("live smoke media must have a supported extension");

    match extension.as_str() {
        "jpg" | "jpeg" | "png" | "webp" | "heic" | "heif" | "gif" => MediaKind::Image,
        "mp4" | "mov" | "webm" => MediaKind::Video,
        _ => panic!("unsupported Gemini live-smoke media extension: .{extension}"),
    }
}
