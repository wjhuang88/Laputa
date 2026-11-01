//! Opt-in live smoke for the Runway video-generation adapter.
//!
//! The test is ignored by default because it consumes external API credits. Credentials
//! are read only from the environment and are never persisted in project files.

use std::env;

use laputa_provider::{GenerationProviderStatus, VideoGenerationProvider, VideoGenerationRequest};
use laputa_provider_runway::{RunwayConfig, RunwayVideoGenerationProvider};

#[tokio::test]
#[ignore = "requires RUNWAYML_API_SECRET and consumes Runway Dev credits"]
async fn live_text_to_video_smoke() {
    let api_key =
        env::var("RUNWAYML_API_SECRET").expect("RUNWAYML_API_SECRET must be set for live smoke");
    let config = RunwayConfig::new(api_key).expect("Runway live config should be valid");
    let provider =
        RunwayVideoGenerationProvider::new(config).expect("Runway live provider should initialize");
    let request = VideoGenerationRequest::new(
        "A calm sunrise over a distant mountain landscape, natural cinematic motion",
        Vec::new(),
    )
    .expect("live smoke request should be valid");

    let handle = provider
        .submit(&request)
        .await
        .expect("Runway live submission should succeed");

    for _ in 0..120 {
        match provider
            .poll(&handle)
            .await
            .expect("Runway live polling should succeed")
        {
            GenerationProviderStatus::Submitted | GenerationProviderStatus::Running => {}
            GenerationProviderStatus::Completed { output } => {
                assert!(
                    output.locator.starts_with("https://"),
                    "Runway live output must be HTTPS"
                );
                return;
            }
            GenerationProviderStatus::Failed { message } => {
                panic!("Runway live generation failed: {message}");
            }
            GenerationProviderStatus::Cancelled => {
                panic!("Runway live generation was cancelled");
            }
        }
    }

    panic!("Runway live smoke exhausted its polling budget");
}
