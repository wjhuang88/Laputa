use anyhow::{Result, ensure};
use laputa_render::{PreviewArtifact, PreviewDecodeRequest, PreviewFrame};

use crate::workspace_state::WorkspaceState;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PreviewPlayerPhase {
    Ready,
    Playing,
    Paused,
    Finished,
    Failed,
}

#[derive(Clone, Debug)]
pub struct PreviewPlayerState {
    artifact: PreviewArtifact,
    phase: PreviewPlayerPhase,
    last_frame_ms: Option<u64>,
    error: Option<String>,
}

impl PreviewPlayerState {
    pub fn new(artifact: PreviewArtifact) -> Result<Self> {
        ensure!(
            artifact.duration_ms > 0,
            "preview artifact duration must be positive"
        );
        ensure!(
            artifact.width > 0,
            "preview artifact width must be positive"
        );
        ensure!(
            artifact.height > 0,
            "preview artifact height must be positive"
        );
        ensure!(
            artifact.fps > 0,
            "preview artifact frame rate must be positive"
        );
        ensure!(
            !artifact.path.as_os_str().is_empty(),
            "preview artifact path must not be empty"
        );
        Ok(Self {
            artifact,
            phase: PreviewPlayerPhase::Ready,
            last_frame_ms: None,
            error: None,
        })
    }

    #[must_use]
    pub fn artifact(&self) -> &PreviewArtifact {
        &self.artifact
    }

    #[must_use]
    pub fn phase(&self) -> PreviewPlayerPhase {
        self.phase
    }

    #[must_use]
    pub fn last_frame_ms(&self) -> Option<u64> {
        self.last_frame_ms
    }

    #[must_use]
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn play(&mut self, workspace: &mut WorkspaceState) -> PreviewDecodeRequest {
        if workspace.playhead_ms() >= self.artifact.duration_ms {
            workspace.seek(0);
        }
        workspace.set_playing(true);
        self.phase = PreviewPlayerPhase::Playing;
        self.error = None;
        self.decode_request(workspace.playhead_ms())
    }

    pub fn pause(&mut self, workspace: &mut WorkspaceState) {
        workspace.set_playing(false);
        self.phase = PreviewPlayerPhase::Paused;
    }

    pub fn seek(&mut self, workspace: &mut WorkspaceState, timeline_ms: u64) {
        workspace.seek(timeline_ms.min(self.artifact.duration_ms));
        workspace.set_playing(false);
        self.phase = PreviewPlayerPhase::Paused;
        self.error = None;
    }

    #[must_use]
    pub fn decode_request(&self, start_ms: u64) -> PreviewDecodeRequest {
        let start_ms = start_ms.min(self.artifact.duration_ms.saturating_sub(1));
        PreviewDecodeRequest {
            input_path: self.artifact.path.clone(),
            start_ms,
            end_ms: self.artifact.duration_ms,
            width: self.artifact.width,
            height: self.artifact.height,
            fps: self.artifact.fps,
        }
    }

    pub fn apply_frame(
        &mut self,
        workspace: &mut WorkspaceState,
        frame: &PreviewFrame,
    ) -> Result<()> {
        ensure!(
            frame.width == self.artifact.width && frame.height == self.artifact.height,
            "decoded preview frame dimensions do not match the preview artifact"
        );
        ensure!(
            frame.timeline_ms <= self.artifact.duration_ms,
            "decoded preview frame is outside the preview duration"
        );
        let expected_len = usize::try_from(frame.width)
            .ok()
            .and_then(|width| {
                usize::try_from(frame.height)
                    .ok()
                    .and_then(|height| width.checked_mul(height))
            })
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or_else(|| anyhow::anyhow!("decoded preview frame dimensions overflow memory"))?;
        ensure!(
            frame.rgba.len() == expected_len,
            "decoded preview frame has an invalid RGBA byte length"
        );

        workspace.seek(frame.timeline_ms);
        self.last_frame_ms = Some(frame.timeline_ms);
        if workspace.is_playing() {
            self.phase = PreviewPlayerPhase::Playing;
        }
        Ok(())
    }

    pub fn finish(&mut self, workspace: &mut WorkspaceState) {
        workspace.seek(self.artifact.duration_ms);
        workspace.set_playing(false);
        self.phase = PreviewPlayerPhase::Finished;
    }

    pub fn fail(&mut self, workspace: &mut WorkspaceState, message: impl Into<String>) {
        workspace.set_playing(false);
        self.phase = PreviewPlayerPhase::Failed;
        self.error = Some(message.into());
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use anyhow::Result;
    use laputa_domain::{Asset, AssetId, AssetTechnicalMetadata, MediaKind, ProjectManifest};
    use laputa_render::{PreviewArtifact, PreviewFrame};
    use laputa_timeline::{
        FilmDraft, FilmState, IntentDraft, SceneDraft, SceneSource, StoryBeatDraft, StoryDraft,
    };

    use super::{PreviewPlayerPhase, PreviewPlayerState};
    use crate::workspace_state::{DefaultEditTarget, WorkspaceState};

    fn workspace() -> Result<WorkspaceState> {
        let photo_id = AssetId::new();
        let video_id = AssetId::new();
        let mut project = ProjectManifest::new("Trip");
        project.assets.push(Asset {
            id: photo_id,
            original_path: PathBuf::from("photo.jpg"),
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
            original_path: PathBuf::from("clip.mp4"),
            display_name: "clip.mp4".to_owned(),
            media_kind: MediaKind::Video,
            source_sha256: "video".to_owned(),
            byte_len: 1,
            technical: AssetTechnicalMetadata {
                width: Some(1920),
                height: Some(1080),
                duration_ms: Some(8_000),
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
                            start_ms: 1_000,
                            end_ms: 5_000,
                        },
                    },
                ],
            },
            &project,
        )?;
        WorkspaceState::from_film(&project, film)
    }

    fn artifact() -> PreviewArtifact {
        PreviewArtifact {
            path: PathBuf::from("preview.mp4"),
            duration_ms: 6_000,
            byte_len: 1,
            width: 2,
            height: 1,
            fps: 2,
            has_audio: false,
        }
    }

    #[test]
    fn playback_advances_playhead_without_changing_edit_selection() -> Result<()> {
        let mut workspace = workspace()?;
        let selected = workspace.scene_strip()[1].scene_id;
        workspace.select_scene(selected)?;
        let mut player = PreviewPlayerState::new(artifact())?;
        let request = player.play(&mut workspace);

        assert_eq!(request.start_ms, 2_000);
        assert_eq!(
            workspace.command_context().default_target,
            DefaultEditTarget::Scene(selected)
        );
        let frame = PreviewFrame {
            timeline_ms: 3_500,
            width: 2,
            height: 1,
            rgba: vec![0; 8],
        };
        player.apply_frame(&mut workspace, &frame)?;

        assert_eq!(workspace.playhead_ms(), 3_500);
        assert_eq!(
            workspace.command_context().default_target,
            DefaultEditTarget::Scene(selected)
        );
        assert_eq!(player.last_frame_ms(), Some(3_500));
        assert_eq!(player.phase(), PreviewPlayerPhase::Playing);
        Ok(())
    }

    #[test]
    fn seek_pauses_and_preserves_selection() -> Result<()> {
        let mut workspace = workspace()?;
        let selected = workspace.scene_strip()[0].scene_id;
        workspace.select_scene(selected)?;
        let mut player = PreviewPlayerState::new(artifact())?;
        let _ = player.play(&mut workspace);

        player.seek(&mut workspace, 4_500);

        assert_eq!(workspace.playhead_ms(), 4_500);
        assert!(!workspace.is_playing());
        assert_eq!(
            workspace.command_context().default_target,
            DefaultEditTarget::Scene(selected)
        );
        assert_eq!(player.phase(), PreviewPlayerPhase::Paused);
        Ok(())
    }

    #[test]
    fn replay_from_finished_position_restarts_from_zero() -> Result<()> {
        let mut workspace = workspace()?;
        let mut player = PreviewPlayerState::new(artifact())?;
        player.finish(&mut workspace);
        assert_eq!(workspace.playhead_ms(), 6_000);

        let request = player.play(&mut workspace);

        assert_eq!(request.start_ms, 0);
        assert_eq!(workspace.playhead_ms(), 0);
        assert!(workspace.is_playing());
        assert_eq!(player.phase(), PreviewPlayerPhase::Playing);
        Ok(())
    }

    #[test]
    fn invalid_frame_is_rejected_before_playhead_changes() -> Result<()> {
        let mut workspace = workspace()?;
        workspace.seek(1_000);
        let mut player = PreviewPlayerState::new(artifact())?;
        let frame = PreviewFrame {
            timeline_ms: 2_000,
            width: 2,
            height: 1,
            rgba: vec![0; 7],
        };

        let result = player.apply_frame(&mut workspace, &frame);

        assert!(result.is_err());
        assert_eq!(workspace.playhead_ms(), 1_000);
        assert_eq!(player.last_frame_ms(), None);
        Ok(())
    }
}
