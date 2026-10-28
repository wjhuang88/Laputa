use anyhow::Result;
use laputa_render::{PreviewDecodeRequest, PreviewFrame};

use crate::{
    preview_playback::{PreviewPlaybackEvent, PreviewPlaybackHandle},
    preview_player::PreviewPlayerState,
    workspace_state::WorkspaceState,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PreviewRunMode {
    Idle,
    Playing,
    SeekPreview { requested_ms: u64 },
}

#[derive(Debug)]
pub struct PreviewController {
    player: PreviewPlayerState,
    generation: u64,
    playback: Option<PreviewPlaybackHandle>,
    mode: PreviewRunMode,
}

impl PreviewController {
    #[must_use]
    pub fn new(player: PreviewPlayerState) -> Self {
        Self {
            player,
            generation: 0,
            playback: None,
            mode: PreviewRunMode::Idle,
        }
    }

    #[must_use]
    pub fn player(&self) -> &PreviewPlayerState {
        &self.player
    }

    #[must_use]
    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn prepare_play(&mut self, workspace: &mut WorkspaceState) -> (u64, PreviewDecodeRequest) {
        self.cancel_current();
        let generation = self.next_generation();
        let request = self.player.play(workspace);
        self.mode = PreviewRunMode::Playing;
        (generation, request)
    }

    pub fn prepare_seek(
        &mut self,
        workspace: &mut WorkspaceState,
        timeline_ms: u64,
    ) -> Option<(u64, PreviewDecodeRequest)> {
        self.cancel_current();
        let generation = self.next_generation();
        let duration_ms = self.player.artifact().duration_ms;
        let requested_ms = timeline_ms.min(duration_ms);
        self.player.seek(workspace, requested_ms);
        self.mode = PreviewRunMode::SeekPreview { requested_ms };

        if requested_ms >= duration_ms {
            self.mode = PreviewRunMode::Idle;
            return None;
        }

        Some((generation, self.player.decode_request(requested_ms)))
    }

    pub fn pause(&mut self, workspace: &mut WorkspaceState) {
        self.cancel_current();
        self.next_generation();
        self.player.pause(workspace);
        self.mode = PreviewRunMode::Idle;
    }

    pub fn attach_playback(&mut self, generation: u64, handle: PreviewPlaybackHandle) {
        if generation == self.generation && !matches!(self.mode, PreviewRunMode::Idle) {
            self.playback = Some(handle);
        }
    }

    pub fn apply_event(
        &mut self,
        workspace: &mut WorkspaceState,
        generation: u64,
        event: PreviewPlaybackEvent,
    ) -> Result<Option<PreviewFrame>> {
        if generation != self.generation {
            return Ok(None);
        }

        match event {
            PreviewPlaybackEvent::Frame(frame) => {
                self.player.apply_frame(workspace, &frame)?;
                if let PreviewRunMode::SeekPreview { requested_ms } = self.mode {
                    workspace.seek(requested_ms);
                    self.cancel_current();
                    // The decoder worker may already have queued another frame before observing
                    // cancellation. Invalidate this generation immediately so those queued events
                    // cannot move the exact seek playhead or mutate player state afterward.
                    self.next_generation();
                    self.mode = PreviewRunMode::Idle;
                }
                Ok(Some(frame))
            }
            PreviewPlaybackEvent::Finished => {
                if matches!(self.mode, PreviewRunMode::Playing) {
                    self.player.finish(workspace);
                }
                self.playback = None;
                self.mode = PreviewRunMode::Idle;
                Ok(None)
            }
            PreviewPlaybackEvent::Failed(message) => {
                self.player.fail(workspace, message);
                self.playback = None;
                self.mode = PreviewRunMode::Idle;
                Ok(None)
            }
        }
    }

    fn cancel_current(&mut self) {
        if let Some(handle) = self.playback.take() {
            handle.cancel();
        }
    }

    fn next_generation(&mut self) -> u64 {
        self.generation = self.generation.wrapping_add(1);
        self.generation
    }
}

impl Drop for PreviewController {
    fn drop(&mut self) {
        self.cancel_current();
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

    use super::PreviewController;
    use crate::{
        preview_playback::PreviewPlaybackEvent,
        preview_player::PreviewPlayerState,
        workspace_state::{DefaultEditTarget, WorkspaceState},
    };

    fn workspace() -> Result<WorkspaceState> {
        let asset_id = AssetId::new();
        let mut project = ProjectManifest::new("Trip");
        project.assets.push(Asset {
            id: asset_id,
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
        let film = FilmState::from_draft(
            "Make a short trip",
            FilmDraft {
                intent: IntentDraft {
                    summary: "Short trip".to_owned(),
                    target_duration_ms: Some(4_000),
                    tone_tags: vec!["warm".to_owned()],
                },
                story: StoryDraft {
                    summary: "One memory".to_owned(),
                    beats: vec![StoryBeatDraft {
                        key: "opening".to_owned(),
                        title: "Opening".to_owned(),
                        purpose: "Establish".to_owned(),
                    }],
                },
                scenes: vec![SceneDraft {
                    beat_key: "opening".to_owned(),
                    asset_id,
                    title: "Opening photo".to_owned(),
                    source: SceneSource::Image { duration_ms: 4_000 },
                }],
            },
            &project,
        )?;
        WorkspaceState::from_film(&project, film)
    }

    fn controller() -> Result<PreviewController> {
        Ok(PreviewController::new(PreviewPlayerState::new(
            PreviewArtifact {
                path: PathBuf::from("preview.mp4"),
                duration_ms: 4_000,
                byte_len: 1,
                width: 2,
                height: 1,
                fps: 2,
                has_audio: false,
            },
        )?))
    }

    fn frame(timeline_ms: u64, value: u8) -> PreviewPlaybackEvent {
        PreviewPlaybackEvent::Frame(PreviewFrame {
            timeline_ms,
            width: 2,
            height: 1,
            rgba: vec![value; 8],
        })
    }

    #[test]
    fn stale_generation_frames_are_ignored_without_state_changes() -> Result<()> {
        let mut workspace = workspace()?;
        let selected = workspace.scene_strip()[0].scene_id;
        workspace.select_scene(selected)?;
        let mut controller = controller()?;
        let (old_generation, _) = controller.prepare_play(&mut workspace);
        controller.pause(&mut workspace);
        let playhead = workspace.playhead_ms();

        let returned = controller.apply_event(&mut workspace, old_generation, frame(3_000, 1))?;

        assert!(returned.is_none());
        assert_eq!(workspace.playhead_ms(), playhead);
        assert_eq!(
            workspace.command_context().default_target,
            DefaultEditTarget::Scene(selected)
        );
        Ok(())
    }

    #[test]
    fn seek_preview_consumes_one_frame_but_preserves_exact_requested_playhead() -> Result<()> {
        let mut workspace = workspace()?;
        let selected = workspace.scene_strip()[0].scene_id;
        workspace.select_scene(selected)?;
        let mut controller = controller()?;
        let (generation, request) = controller
            .prepare_seek(&mut workspace, 1_750)
            .expect("non-terminal seek should request one preview frame");

        assert_eq!(request.start_ms, 1_750);
        let returned = controller.apply_event(&mut workspace, generation, frame(1_700, 2))?;

        assert!(returned.is_some());
        assert_eq!(workspace.playhead_ms(), 1_750);
        assert!(!workspace.is_playing());
        assert_eq!(
            workspace.command_context().default_target,
            DefaultEditTarget::Scene(selected)
        );
        Ok(())
    }

    #[test]
    fn queued_frames_after_seek_preview_are_fenced_from_exact_playhead() -> Result<()> {
        let mut workspace = workspace()?;
        let selected = workspace.scene_strip()[0].scene_id;
        workspace.select_scene(selected)?;
        let mut controller = controller()?;
        let (seek_generation, _) = controller
            .prepare_seek(&mut workspace, 1_750)
            .expect("non-terminal seek should request preview frames");

        let first = controller.apply_event(&mut workspace, seek_generation, frame(1_700, 2))?;
        let fenced_generation = controller.generation();
        let queued = controller.apply_event(&mut workspace, seek_generation, frame(2_000, 3))?;

        assert!(first.is_some());
        assert!(queued.is_none());
        assert_ne!(fenced_generation, seek_generation);
        assert_eq!(workspace.playhead_ms(), 1_750);
        assert_eq!(
            workspace.command_context().default_target,
            DefaultEditTarget::Scene(selected)
        );
        Ok(())
    }

    #[test]
    fn seeking_to_end_does_not_start_a_decoder_generation() -> Result<()> {
        let mut workspace = workspace()?;
        let mut controller = controller()?;

        let request = controller.prepare_seek(&mut workspace, 4_000);

        assert!(request.is_none());
        assert_eq!(workspace.playhead_ms(), 4_000);
        assert!(!workspace.is_playing());
        Ok(())
    }
}
