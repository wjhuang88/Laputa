use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use laputa_domain::{AssetId, MediaKind, ProjectManifest};
use laputa_timeline::{FilmState, SceneId, SceneSource};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PreviewSource {
    Image {
        path: PathBuf,
    },
    Video {
        path: PathBuf,
        source_start_ms: u64,
        source_end_ms: u64,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreviewSegment {
    pub scene_id: SceneId,
    pub asset_id: AssetId,
    pub timeline_start_ms: u64,
    pub timeline_end_ms: u64,
    pub source: PreviewSource,
}

impl PreviewSegment {
    #[must_use]
    pub fn duration_ms(&self) -> u64 {
        self.timeline_end_ms - self.timeline_start_ms
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SceneStripItem {
    pub scene_id: SceneId,
    pub asset_id: AssetId,
    pub title: String,
    pub display_name: String,
    pub media_kind: MediaKind,
    pub timeline_start_ms: u64,
    pub timeline_end_ms: u64,
}

impl SceneStripItem {
    #[must_use]
    pub fn duration_ms(&self) -> u64 {
        self.timeline_end_ms - self.timeline_start_ms
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DefaultEditTarget {
    Project,
    Scene(SceneId),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommandContext {
    pub default_target: DefaultEditTarget,
}

#[derive(Clone, Debug)]
pub struct WorkspaceState {
    film: FilmState,
    scene_strip: Vec<SceneStripItem>,
    preview_plan: Vec<PreviewSegment>,
    selected_scene: Option<SceneId>,
    playhead_ms: u64,
    playing: bool,
}

impl WorkspaceState {
    pub fn from_film(project: &ProjectManifest, film: FilmState) -> Result<Self> {
        film.validate_for(project)
            .context("cannot build workspace from invalid film state")?;

        let mut scene_strip = Vec::with_capacity(film.timeline.scenes.len());
        let mut preview_plan = Vec::with_capacity(film.timeline.scenes.len());
        let mut timeline_cursor = 0_u64;

        for scene in &film.timeline.scenes {
            let asset = project.asset(scene.asset_id).with_context(|| {
                format!(
                    "scene {} references asset {} that is not in the project",
                    scene.title, scene.asset_id
                )
            })?;
            let duration_ms = scene.source.duration_ms();
            let timeline_end_ms = timeline_cursor
                .checked_add(duration_ms)
                .context("timeline duration overflow while building workspace")?;

            scene_strip.push(SceneStripItem {
                scene_id: scene.id,
                asset_id: scene.asset_id,
                title: scene.title.clone(),
                display_name: asset.display_name.clone(),
                media_kind: asset.media_kind,
                timeline_start_ms: timeline_cursor,
                timeline_end_ms,
            });

            let source = match &scene.source {
                SceneSource::Image { .. } => PreviewSource::Image {
                    path: asset.original_path.clone(),
                },
                SceneSource::Video { start_ms, end_ms } => PreviewSource::Video {
                    path: asset.original_path.clone(),
                    source_start_ms: *start_ms,
                    source_end_ms: *end_ms,
                },
            };
            preview_plan.push(PreviewSegment {
                scene_id: scene.id,
                asset_id: scene.asset_id,
                timeline_start_ms: timeline_cursor,
                timeline_end_ms,
                source,
            });
            timeline_cursor = timeline_end_ms;
        }

        if scene_strip.is_empty() {
            bail!("workspace requires at least one scene");
        }

        Ok(Self {
            film,
            scene_strip,
            preview_plan,
            selected_scene: None,
            playhead_ms: 0,
            playing: false,
        })
    }

    #[must_use]
    pub fn film(&self) -> &FilmState {
        &self.film
    }

    #[must_use]
    pub fn scene_strip(&self) -> &[SceneStripItem] {
        &self.scene_strip
    }

    #[must_use]
    pub fn preview_plan(&self) -> &[PreviewSegment] {
        &self.preview_plan
    }

    #[must_use]
    pub fn duration_ms(&self) -> u64 {
        self.preview_plan
            .last()
            .map_or(0, |segment| segment.timeline_end_ms)
    }

    #[must_use]
    pub fn playhead_ms(&self) -> u64 {
        self.playhead_ms
    }

    #[must_use]
    pub fn is_playing(&self) -> bool {
        self.playing
    }

    pub fn set_playing(&mut self, playing: bool) {
        self.playing = playing;
    }

    pub fn toggle_playback(&mut self) {
        self.playing = !self.playing;
    }

    pub fn seek(&mut self, timeline_ms: u64) {
        self.playhead_ms = timeline_ms.min(self.duration_ms());
    }

    #[must_use]
    pub fn selected_scene(&self) -> Option<SceneId> {
        self.selected_scene
    }

    pub fn select_scene(&mut self, scene_id: SceneId) -> Result<()> {
        let scene = self
            .scene_strip
            .iter()
            .find(|scene| scene.scene_id == scene_id)
            .with_context(|| format!("scene {scene_id:?} is not in the workspace"))?;
        self.selected_scene = Some(scene_id);
        self.playhead_ms = scene.timeline_start_ms;
        Ok(())
    }

    pub fn clear_selection(&mut self) {
        self.selected_scene = None;
    }

    #[must_use]
    pub fn command_context(&self) -> CommandContext {
        CommandContext {
            default_target: self
                .selected_scene
                .map_or(DefaultEditTarget::Project, DefaultEditTarget::Scene),
        }
    }

    #[must_use]
    pub fn scene_at_playhead(&self) -> &SceneStripItem {
        if self.playhead_ms >= self.duration_ms() {
            return self
                .scene_strip
                .last()
                .expect("workspace construction guarantees a scene");
        }

        self.scene_strip
            .iter()
            .find(|scene| {
                self.playhead_ms >= scene.timeline_start_ms
                    && self.playhead_ms < scene.timeline_end_ms
            })
            .expect("validated workspace timeline must cover the playhead")
    }

    #[must_use]
    pub fn preview_segment_at_playhead(&self) -> &PreviewSegment {
        let scene_id = self.scene_at_playhead().scene_id;
        self.preview_plan
            .iter()
            .find(|segment| segment.scene_id == scene_id)
            .expect("scene strip and preview plan are built together")
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use anyhow::Result;
    use laputa_domain::{Asset, AssetId, AssetTechnicalMetadata, MediaKind, ProjectManifest};
    use laputa_timeline::{
        FilmDraft, FilmState, IntentDraft, SceneDraft, SceneSource, StoryBeatDraft, StoryDraft,
    };

    use super::{DefaultEditTarget, PreviewSource, WorkspaceState};

    fn fixture() -> Result<(ProjectManifest, FilmState)> {
        let photo_id = AssetId::new();
        let video_id = AssetId::new();
        let mut project = ProjectManifest::new("Kyoto Trip");
        project.assets.push(Asset {
            id: photo_id,
            original_path: PathBuf::from("/fixtures/temple.jpg"),
            display_name: "temple.jpg".to_owned(),
            media_kind: MediaKind::Image,
            source_sha256: "photo-revision".to_owned(),
            byte_len: 10,
            technical: AssetTechnicalMetadata {
                width: Some(1600),
                height: Some(900),
                ..AssetTechnicalMetadata::default()
            },
        });
        project.assets.push(Asset {
            id: video_id,
            original_path: PathBuf::from("/fixtures/street.mp4"),
            display_name: "street.mp4".to_owned(),
            media_kind: MediaKind::Video,
            source_sha256: "video-revision".to_owned(),
            byte_len: 20,
            technical: AssetTechnicalMetadata {
                width: Some(1920),
                height: Some(1080),
                duration_ms: Some(8_000),
                video_codec: Some("fixture".to_owned()),
                has_audio: true,
            },
        });

        let draft = FilmDraft {
            intent: IntentDraft {
                summary: "Warm Kyoto memory".to_owned(),
                target_duration_ms: Some(9_000),
                tone_tags: vec!["warm".to_owned()],
            },
            story: StoryDraft {
                summary: "Arrive, then walk through Kyoto".to_owned(),
                beats: vec![
                    StoryBeatDraft {
                        key: "arrival".to_owned(),
                        title: "Arrival".to_owned(),
                        purpose: "Establish place".to_owned(),
                    },
                    StoryBeatDraft {
                        key: "walk".to_owned(),
                        title: "Walk".to_owned(),
                        purpose: "Add movement".to_owned(),
                    },
                ],
            },
            scenes: vec![
                SceneDraft {
                    beat_key: "arrival".to_owned(),
                    asset_id: photo_id,
                    title: "Temple portrait".to_owned(),
                    source: SceneSource::Image { duration_ms: 3_000 },
                },
                SceneDraft {
                    beat_key: "walk".to_owned(),
                    asset_id: video_id,
                    title: "Street walk".to_owned(),
                    source: SceneSource::Video {
                        start_ms: 1_000,
                        end_ms: 7_000,
                    },
                },
            ],
        };
        let film = FilmState::from_draft("Make a warm Kyoto vlog", draft, &project)?;
        Ok((project, film))
    }

    #[test]
    fn film_projects_to_ordered_scene_strip_and_preview_plan() -> Result<()> {
        let (project, film) = fixture()?;
        let state = WorkspaceState::from_film(&project, film)?;

        assert_eq!(state.duration_ms(), 9_000);
        assert_eq!(state.scene_strip().len(), 2);
        assert_eq!(state.scene_strip()[0].title, "Temple portrait");
        assert_eq!(state.scene_strip()[0].timeline_start_ms, 0);
        assert_eq!(state.scene_strip()[0].timeline_end_ms, 3_000);
        assert_eq!(state.scene_strip()[1].timeline_start_ms, 3_000);
        assert_eq!(state.scene_strip()[1].timeline_end_ms, 9_000);
        assert_eq!(state.preview_plan().len(), 2);
        assert_eq!(
            state.preview_plan()[1].source,
            PreviewSource::Video {
                path: PathBuf::from("/fixtures/street.mp4"),
                source_start_ms: 1_000,
                source_end_ms: 7_000,
            }
        );
        Ok(())
    }

    #[test]
    fn seek_tracks_current_scene_without_changing_edit_selection() -> Result<()> {
        let (project, film) = fixture()?;
        let mut state = WorkspaceState::from_film(&project, film)?;
        let first_scene = state.scene_strip()[0].scene_id;

        state.select_scene(first_scene)?;
        state.seek(3_000);

        assert_eq!(state.scene_at_playhead().title, "Street walk");
        assert_eq!(state.selected_scene(), Some(first_scene));
        assert_eq!(
            state.command_context().default_target,
            DefaultEditTarget::Scene(first_scene)
        );
        Ok(())
    }

    #[test]
    fn selecting_scene_sets_default_command_target_and_seeks_to_scene_start() -> Result<()> {
        let (project, film) = fixture()?;
        let mut state = WorkspaceState::from_film(&project, film)?;
        let second = state.scene_strip()[1].clone();

        state.select_scene(second.scene_id)?;

        assert_eq!(state.playhead_ms(), second.timeline_start_ms);
        assert_eq!(
            state.command_context().default_target,
            DefaultEditTarget::Scene(second.scene_id)
        );
        state.clear_selection();
        assert_eq!(
            state.command_context().default_target,
            DefaultEditTarget::Project
        );
        Ok(())
    }

    #[test]
    fn seek_clamps_to_duration_and_uses_last_scene_at_end() -> Result<()> {
        let (project, film) = fixture()?;
        let mut state = WorkspaceState::from_film(&project, film)?;

        state.seek(u64::MAX);

        assert_eq!(state.playhead_ms(), 9_000);
        assert_eq!(state.scene_at_playhead().title, "Street walk");
        assert_eq!(state.preview_segment_at_playhead().timeline_end_ms, 9_000);
        Ok(())
    }

    #[test]
    fn workspace_revalidates_film_before_projection() -> Result<()> {
        let (project, mut film) = fixture()?;
        film.timeline.scenes[0].asset_id = AssetId::new();

        let error = WorkspaceState::from_film(&project, film)
            .expect_err("invalid film must not be projected into UI state");

        assert!(error.to_string().contains("invalid film state"));
        Ok(())
    }
}
