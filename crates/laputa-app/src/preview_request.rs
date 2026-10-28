use std::path::PathBuf;

use laputa_render::{PreviewClip, PreviewRenderRequest};

use crate::workspace_state::{PreviewSource, WorkspaceState};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PreviewProfile {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
}

impl Default for PreviewProfile {
    fn default() -> Self {
        Self {
            width: 960,
            height: 540,
            fps: 30,
        }
    }
}

#[must_use]
pub fn preview_render_request(
    workspace: &WorkspaceState,
    output_path: impl Into<PathBuf>,
    profile: PreviewProfile,
) -> PreviewRenderRequest {
    let clips = workspace
        .preview_plan()
        .iter()
        .map(|segment| match &segment.source {
            PreviewSource::Image { path } => PreviewClip::Image {
                path: path.clone(),
                duration_ms: segment.duration_ms(),
            },
            PreviewSource::Video {
                path,
                source_start_ms,
                source_end_ms,
            } => PreviewClip::Video {
                path: path.clone(),
                source_start_ms: *source_start_ms,
                source_end_ms: *source_end_ms,
            },
        })
        .collect();

    PreviewRenderRequest {
        clips,
        output_path: output_path.into(),
        width: profile.width,
        height: profile.height,
        fps: profile.fps,
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use anyhow::Result;
    use laputa_domain::{Asset, AssetId, AssetTechnicalMetadata, MediaKind, ProjectManifest};
    use laputa_render::PreviewClip;
    use laputa_timeline::{
        FilmDraft, FilmState, IntentDraft, SceneDraft, SceneSource, StoryBeatDraft, StoryDraft,
    };

    use super::{PreviewProfile, preview_render_request};
    use crate::workspace_state::WorkspaceState;

    #[test]
    fn workspace_projection_maps_to_deterministic_render_request() -> Result<()> {
        let photo_id = AssetId::new();
        let video_id = AssetId::new();
        let photo = PathBuf::from("/fixtures/photo.jpg");
        let video = PathBuf::from("/fixtures/video.mp4");
        let mut project = ProjectManifest::new("Trip");
        project.assets.push(Asset {
            id: photo_id,
            original_path: photo.clone(),
            display_name: "photo.jpg".to_owned(),
            media_kind: MediaKind::Image,
            source_sha256: "photo".to_owned(),
            byte_len: 1,
            technical: AssetTechnicalMetadata {
                width: Some(1600),
                height: Some(900),
                ..AssetTechnicalMetadata::default()
            },
        });
        project.assets.push(Asset {
            id: video_id,
            original_path: video.clone(),
            display_name: "video.mp4".to_owned(),
            media_kind: MediaKind::Video,
            source_sha256: "video".to_owned(),
            byte_len: 1,
            technical: AssetTechnicalMetadata {
                width: Some(1920),
                height: Some(1080),
                duration_ms: Some(7_000),
                video_codec: Some("fixture".to_owned()),
                has_audio: true,
            },
        });
        let film = FilmState::from_draft(
            "Make a short trip",
            FilmDraft {
                intent: IntentDraft {
                    summary: "Short trip".to_owned(),
                    target_duration_ms: Some(6_000),
                    tone_tags: vec!["warm".to_owned()],
                },
                story: StoryDraft {
                    summary: "Photo then motion".to_owned(),
                    beats: vec![
                        StoryBeatDraft {
                            key: "photo".to_owned(),
                            title: "Photo".to_owned(),
                            purpose: "Open".to_owned(),
                        },
                        StoryBeatDraft {
                            key: "motion".to_owned(),
                            title: "Motion".to_owned(),
                            purpose: "Move".to_owned(),
                        },
                    ],
                },
                scenes: vec![
                    SceneDraft {
                        beat_key: "photo".to_owned(),
                        asset_id: photo_id,
                        title: "Opening".to_owned(),
                        source: SceneSource::Image { duration_ms: 2_000 },
                    },
                    SceneDraft {
                        beat_key: "motion".to_owned(),
                        asset_id: video_id,
                        title: "Walk".to_owned(),
                        source: SceneSource::Video {
                            start_ms: 1_500,
                            end_ms: 5_500,
                        },
                    },
                ],
            },
            &project,
        )?;
        let workspace = WorkspaceState::from_film(&project, film)?;

        let request = preview_render_request(
            &workspace,
            "/project/.laputa/cache/preview/current.mp4",
            PreviewProfile::default(),
        );

        assert_eq!(request.width, 960);
        assert_eq!(request.height, 540);
        assert_eq!(request.fps, 30);
        assert_eq!(request.duration_ms(), 6_000);
        assert_eq!(
            request.clips,
            vec![
                PreviewClip::Image {
                    path: photo,
                    duration_ms: 2_000,
                },
                PreviewClip::Video {
                    path: video,
                    source_start_ms: 1_500,
                    source_end_ms: 5_500,
                },
            ]
        );
        Ok(())
    }
}
