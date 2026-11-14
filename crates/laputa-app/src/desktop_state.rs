use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};
use laputa_domain::{Asset, AssetId, MediaKind, ProjectManifest};
use laputa_timeline::{
    FilmState, FilmStore, SceneId,
    revision::{RevisionAction, RevisionOrigin, shortened_source},
};

use crate::workspace_state::WorkspaceState;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum AssetFilter {
    #[default]
    All,
    Photos,
    Videos,
}

impl AssetFilter {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::All => "全部",
            Self::Photos => "照片",
            Self::Videos => "视频",
        }
    }

    fn matches(self, kind: MediaKind) -> bool {
        match self {
            Self::All => true,
            Self::Photos => kind == MediaKind::Image,
            Self::Videos => kind == MediaKind::Video,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum DesktopOperation {
    #[default]
    Idle,
    ChoosingProject,
    CreatingProject,
    OpeningProject,
    ChoosingMedia,
    ImportingMedia,
}

impl DesktopOperation {
    #[must_use]
    pub const fn is_busy(self) -> bool {
        !matches!(self, Self::Idle)
    }

    #[must_use]
    pub const fn label(self) -> Option<&'static str> {
        match self {
            Self::Idle => None,
            Self::ChoosingProject => Some("选择项目位置…"),
            Self::CreatingProject => Some("正在创建影片…"),
            Self::OpeningProject => Some("正在打开影片…"),
            Self::ChoosingMedia => Some("选择照片和视频…"),
            Self::ImportingMedia => Some("正在添加素材…"),
        }
    }
}

/// An immediate, direct user action on a known Scene ID. No Agent turn required.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DirectSceneEdit {
    MoveEarlier(SceneId),
    MoveLater(SceneId),
    /// Pointer drop: insert this Scene immediately before target, or at the end.
    ReorderBefore {
        scene_id: SceneId,
        before: Option<SceneId>,
    },
    Shorten(SceneId),
    Delete(SceneId),
    SetLocked(SceneId, bool),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetCard {
    pub id: AssetId,
    pub display_name: String,
    pub kind: MediaKind,
    pub kind_label: &'static str,
    pub detail: String,
}

impl AssetCard {
    fn from_asset(asset: &Asset) -> Self {
        Self {
            id: asset.id,
            display_name: asset.display_name.clone(),
            kind: asset.media_kind,
            kind_label: match asset.media_kind {
                MediaKind::Image => "照片",
                MediaKind::Video => "视频",
            },
            detail: asset_detail(asset),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct DesktopState {
    project_root: Option<PathBuf>,
    project: Option<ProjectManifest>,
    workspace: Option<WorkspaceState>,
    filter: AssetFilter,
    operation: DesktopOperation,
    notice: Option<String>,
    error: Option<String>,
}

impl DesktopState {
    #[must_use]
    pub fn launcher() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn from_project(root: impl Into<PathBuf>, project: ProjectManifest) -> Self {
        let root = root.into();
        let mut state = Self {
            project_root: Some(root.clone()),
            project: Some(project.clone()),
            ..Self::default()
        };

        match FilmStore::at(&root).load(&project) {
            Ok(Some(film)) => match WorkspaceState::from_film(&project, film) {
                Ok(workspace) => state.workspace = Some(workspace),
                Err(error) => {
                    state.error = Some(format!("无法恢复影片工作区：{error}"));
                }
            },
            Ok(None) => {}
            Err(error) => {
                state.error = Some(format!("无法读取影片状态：{error}"));
            }
        }
        state
    }

    pub fn from_project_with_film(
        root: impl Into<PathBuf>,
        project: ProjectManifest,
        film: FilmState,
    ) -> Result<Self> {
        let workspace = WorkspaceState::from_film(&project, film)?;
        Ok(Self {
            project_root: Some(root.into()),
            project: Some(project),
            workspace: Some(workspace),
            ..Self::default()
        })
    }

    #[must_use]
    pub fn project(&self) -> Option<&ProjectManifest> {
        self.project.as_ref()
    }

    #[must_use]
    pub fn project_root(&self) -> Option<&Path> {
        self.project_root.as_deref()
    }

    #[must_use]
    pub fn workspace(&self) -> Option<&WorkspaceState> {
        self.workspace.as_ref()
    }

    pub fn workspace_mut(&mut self) -> Option<&mut WorkspaceState> {
        self.workspace.as_mut()
    }

    #[must_use]
    pub fn filter(&self) -> AssetFilter {
        self.filter
    }

    pub fn set_filter(&mut self, filter: AssetFilter) {
        self.filter = filter;
    }

    #[must_use]
    pub fn operation(&self) -> DesktopOperation {
        self.operation
    }

    #[must_use]
    pub fn notice(&self) -> Option<&str> {
        self.notice.as_deref()
    }

    #[must_use]
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    #[must_use]
    pub fn asset_count(&self) -> usize {
        self.project
            .as_ref()
            .map_or(0, |project| project.assets.len())
    }

    #[must_use]
    pub fn asset_cards(&self) -> Vec<AssetCard> {
        self.project
            .as_ref()
            .into_iter()
            .flat_map(|project| project.assets.iter())
            .filter(|asset| self.filter.matches(asset.media_kind))
            .map(AssetCard::from_asset)
            .collect()
    }

    /// Persist a direct edit, then rebuild the Film Workspace projection.
    ///
    /// This v0.1 desktop entrypoint assumes one active project writer. A stale
    /// UI projection is rejected instead of silently editing an unseen revision.
    pub fn apply_direct_scene_edit(&mut self, edit: DirectSceneEdit) -> Result<()> {
        ensure!(
            !self.operation.is_busy(),
            "finish the current project operation before editing"
        );
        let root = self.project_root.as_ref().context("no open project")?;
        let project = self.project.as_ref().context("no open project manifest")?;
        let previous = self
            .workspace
            .as_ref()
            .context("a committed film is required")?;
        let store = FilmStore::at(root);
        let disk_film = store
            .load(project)?
            .context("cannot edit without a persisted first cut")?;
        ensure!(
            disk_film == *previous.film(),
            "film changed outside this workspace; reopen the project before editing"
        );

        let scenes = &disk_film.timeline.scenes;
        let action = match edit {
            DirectSceneEdit::MoveEarlier(scene_id) => {
                let index = scenes
                    .iter()
                    .position(|scene| scene.id == scene_id)
                    .context("scene is missing from the film")?;
                ensure!(index > 0, "scene is already first");
                RevisionAction::Reorder {
                    scene_id,
                    before: Some(scenes[index - 1].id),
                }
            }
            DirectSceneEdit::MoveLater(scene_id) => {
                let index = scenes
                    .iter()
                    .position(|scene| scene.id == scene_id)
                    .context("scene is missing from the film")?;
                ensure!(index + 1 < scenes.len(), "scene is already last");
                RevisionAction::Reorder {
                    scene_id,
                    before: scenes.get(index + 2).map(|scene| scene.id),
                }
            }
            DirectSceneEdit::ReorderBefore { scene_id, before } => {
                RevisionAction::Reorder { scene_id, before }
            }
            DirectSceneEdit::Shorten(scene_id) => {
                let scene = scenes
                    .iter()
                    .find(|scene| scene.id == scene_id)
                    .context("scene is missing from the film")?;
                RevisionAction::Trim {
                    scene_id,
                    source: shortened_source(&scene.source)?,
                }
            }
            DirectSceneEdit::Delete(scene_id) => RevisionAction::Delete { scene_id },
            DirectSceneEdit::SetLocked(scene_id, locked) => {
                RevisionAction::SetLocked { scene_id, locked }
            }
        };

        // Preflight both timeline invariants and the actual GUI preview projection
        // before writing to disk; rejected revisions do not disturb the UI.
        let candidate = disk_film.revised(project, action.clone(), RevisionOrigin::User)?;
        let mut refreshed = WorkspaceState::from_film(project, candidate.clone())?;
        let old_selection = previous.selected_scene();
        let old_playhead = previous.playhead_ms();
        if let Some(scene_id) = old_selection {
            if refreshed
                .scene_strip()
                .iter()
                .any(|scene| scene.scene_id == scene_id)
            {
                refreshed.select_scene(scene_id)?;
            } else {
                refreshed.seek(old_playhead);
            }
        } else {
            refreshed.seek(old_playhead);
        }

        let saved = store.apply_revision(project, action, RevisionOrigin::User)?;
        ensure!(
            saved == candidate,
            "revision result changed during editing; reopen the project"
        );
        self.workspace = Some(refreshed);
        self.notice = Some("影片片段已更新".to_owned());
        self.error = None;
        Ok(())
    }

    pub fn begin(&mut self, operation: DesktopOperation) -> bool {
        if self.operation.is_busy() {
            return false;
        }
        self.operation = operation;
        self.error = None;
        self.notice = None;
        true
    }

    pub fn cancel_operation(&mut self) {
        self.operation = DesktopOperation::Idle;
    }

    pub fn project_loaded(&mut self, root: PathBuf, project: ProjectManifest) {
        *self = Self::from_project(root, project);
    }

    pub fn project_loaded_with_film(
        &mut self,
        root: PathBuf,
        project: ProjectManifest,
        film: FilmState,
    ) -> Result<()> {
        let workspace = WorkspaceState::from_film(&project, film)?;
        self.project_root = Some(root);
        self.project = Some(project);
        self.workspace = Some(workspace);
        self.filter = AssetFilter::All;
        self.operation = DesktopOperation::Idle;
        self.error = None;
        self.notice = None;
        Ok(())
    }

    pub fn import_completed(&mut self, project: ProjectManifest, imported: usize) {
        self.project = Some(project);
        self.operation = DesktopOperation::Idle;
        self.error = None;
        self.notice = Some(format!("已添加 {imported} 个素材"));
    }

    pub fn fail(&mut self, message: impl Into<String>) {
        self.operation = DesktopOperation::Idle;
        self.error = Some(message.into());
        self.notice = None;
    }
}

fn asset_detail(asset: &Asset) -> String {
    let dimensions = match (asset.technical.width, asset.technical.height) {
        (Some(width), Some(height)) => Some(format!("{width}×{height}")),
        _ => None,
    };
    let duration = asset.technical.duration_ms.map(format_duration);

    [duration, dimensions]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" · ")
}

fn format_duration(duration_ms: u64) -> String {
    let total_seconds = duration_ms.div_ceil(1_000);
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    format!("{minutes}:{seconds:02}")
}

#[cfg(test)]
mod tests {
    use std::fs;

    use anyhow::{Context, Result};
    use laputa_domain::{AssetTechnicalMetadata, MediaKind};
    use laputa_media::{MediaProbe, ProjectStore};
    use laputa_timeline::{
        FilmDraft, FilmState, FilmStore, IntentDraft, SceneDraft, SceneSource, StoryBeatDraft,
        StoryDraft,
    };
    use tempfile::tempdir;

    use super::{AssetFilter, DesktopOperation, DesktopState, DirectSceneEdit};

    struct FixtureProbe;

    impl MediaProbe for FixtureProbe {
        fn probe(
            &self,
            _source: &std::path::Path,
            kind: MediaKind,
        ) -> Result<AssetTechnicalMetadata> {
            Ok(match kind {
                MediaKind::Image => AssetTechnicalMetadata {
                    width: Some(1600),
                    height: Some(900),
                    ..AssetTechnicalMetadata::default()
                },
                MediaKind::Video => AssetTechnicalMetadata {
                    width: Some(1920),
                    height: Some(1080),
                    duration_ms: Some(12_400),
                    video_codec: Some("fixture".into()),
                    has_audio: true,
                },
            })
        }
    }

    fn scene_edit_fixture() -> Result<(tempfile::TempDir, DesktopState)> {
        let temp = tempdir()?;
        let photo = temp.path().join("scene-a.jpg");
        let video = temp.path().join("scene-b.mp4");
        fs::write(&photo, b"original-image-sentinel")?;
        fs::write(&video, b"original-video-sentinel")?;
        let root = temp.path().join("story");
        let store = ProjectStore::at(&root);
        store.create("Film")?;
        let report = store.import_media(&[photo, video], &FixtureProbe)?;
        let project = report.project;
        let film = FilmState::from_draft(
            "Simple film",
            FilmDraft {
                intent: IntentDraft {
                    summary: "Memories".to_owned(),
                    target_duration_ms: Some(10_000),
                    tone_tags: vec!["warm".into()],
                },
                story: StoryDraft {
                    summary: "Two moments".into(),
                    beats: vec![
                        StoryBeatDraft {
                            key: "a".into(),
                            title: "Arrival".into(),
                            purpose: "Establish the place".into(),
                        },
                        StoryBeatDraft {
                            key: "b".into(),
                            title: "Walking".into(),
                            purpose: "Show motion".into(),
                        },
                    ],
                },
                scenes: vec![
                    SceneDraft {
                        beat_key: "a".into(),
                        asset_id: report.imported[0].id,
                        title: "Photo".into(),
                        source: SceneSource::Image { duration_ms: 4_000 },
                    },
                    SceneDraft {
                        beat_key: "b".into(),
                        asset_id: report.imported[1].id,
                        title: "Video".into(),
                        source: SceneSource::Video {
                            start_ms: 1_000,
                            end_ms: 7_000,
                        },
                    },
                ],
            },
            &project,
        )?;
        FilmStore::at(&root).save(&project, &film)?;
        Ok((
            temp,
            DesktopState::from_project_with_film(root, project, film)?,
        ))
    }

    #[test]
    fn direct_scene_buttons_persist_order_and_refresh_selection_preview() -> Result<()> {
        let (temp, mut state) = scene_edit_fixture()?;
        let first = state
            .workspace()
            .context("workspace missing")?
            .scene_strip()[0]
            .scene_id;
        let second = state
            .workspace()
            .context("workspace missing")?
            .scene_strip()[1]
            .scene_id;
        state
            .workspace_mut()
            .context("workspace missing")?
            .select_scene(second)?;

        state.apply_direct_scene_edit(DirectSceneEdit::MoveEarlier(second))?;
        let workspace = state.workspace().context("workspace missing")?;
        assert_eq!(workspace.scene_strip()[0].scene_id, second);
        assert_eq!(workspace.selected_scene(), Some(second));
        assert_eq!(workspace.playhead_ms(), 0);
        assert_eq!(workspace.preview_plan()[0].scene_id, second);

        state.apply_direct_scene_edit(DirectSceneEdit::MoveLater(second))?;
        assert_eq!(
            state
                .workspace()
                .context("workspace missing")?
                .scene_strip()[0]
                .scene_id,
            first
        );
        assert_eq!(
            state
                .workspace()
                .context("workspace missing")?
                .preview_plan()[1]
                .scene_id,
            second
        );
        let reopened = super::DesktopState::from_project(
            temp.path().join("story"),
            state.project().context("project missing")?.clone(),
        );
        assert_eq!(
            reopened
                .workspace()
                .context("reopened workspace missing")?
                .scene_strip()[0]
                .scene_id,
            first
        );
        assert_eq!(
            reopened
                .workspace()
                .context("reopened workspace missing")?
                .scene_strip()[1]
                .scene_id,
            second
        );
        Ok(())
    }

    #[test]
    fn direct_drag_reorder_before_end_and_retained_rollback() -> Result<()> {
        let (temp, mut state) = scene_edit_fixture()?;
        let scenes = state.workspace().context("workspace missing")?.scene_strip();
        let first = scenes[0].scene_id;
        let second = scenes[1].scene_id;
        let root = temp.path().join("story");
        let project = state.project().context("project missing")?.clone();
        let store = FilmStore::at(&root);
        let original_a = fs::read(temp.path().join("scene-a.jpg"))?;
        let original_b = fs::read(temp.path().join("scene-b.mp4"))?;

        state.workspace_mut().context("workspace missing")?.select_scene(second)?;
        state.apply_direct_scene_edit(DirectSceneEdit::ReorderBefore {
            scene_id: second,
            before: Some(first),
        })?;
        let workspace = state.workspace().context("workspace missing")?;
        assert_eq!(workspace.scene_strip()[0].scene_id, second);
        assert_eq!(workspace.preview_plan()[0].scene_id, second);
        assert_eq!(workspace.selected_scene(), Some(second));
        let reopened = DesktopState::from_project(&root, project.clone());
        assert_eq!(
            reopened.workspace().context("reopened workspace missing")?.scene_strip()[0].scene_id,
            second
        );

        state.apply_direct_scene_edit(DirectSceneEdit::ReorderBefore {
            scene_id: second,
            before: None,
        })?;
        assert_eq!(
            state.workspace().context("workspace missing")?.scene_strip()[1].scene_id,
            second
        );
        state.apply_direct_scene_edit(DirectSceneEdit::SetLocked(second, true))?;
        let unchanged_film = fs::read(store.path())?;
        assert!(
            state.apply_direct_scene_edit(DirectSceneEdit::ReorderBefore {
                scene_id: second,
                before: Some(first),
            }).is_err()
        );
        assert_eq!(fs::read(store.path())?, unchanged_film);
        assert_eq!(
            state.workspace().context("workspace missing")?.scene_strip()[1].scene_id,
            second
        );
        assert_eq!(fs::read(temp.path().join("scene-a.jpg"))?, original_a);
        assert_eq!(fs::read(temp.path().join("scene-b.mp4"))?, original_b);
        Ok(())
    }

    #[test]
    fn direct_shorten_lock_delete_reopen_and_original_bytes_remain_unchanged() -> Result<()> {
        let (temp, mut state) = scene_edit_fixture()?;
        let first = state
            .workspace()
            .context("workspace missing")?
            .scene_strip()[0]
            .scene_id;
        let second = state
            .workspace()
            .context("workspace missing")?
            .scene_strip()[1]
            .scene_id;
        let store = FilmStore::at(temp.path().join("story"));
        let project = state.project().context("project missing")?.clone();
        let source_a = fs::read(temp.path().join("scene-a.jpg"))?;
        let source_b = fs::read(temp.path().join("scene-b.mp4"))?;

        state.apply_direct_scene_edit(DirectSceneEdit::Shorten(first))?;
        assert_eq!(
            state
                .workspace()
                .context("workspace missing")?
                .scene_strip()[0]
                .duration_ms(),
            3_200
        );
        assert_eq!(
            state
                .workspace()
                .context("workspace missing")?
                .duration_ms(),
            9_200
        );
        state.apply_direct_scene_edit(DirectSceneEdit::SetLocked(first, true))?;
        let saved = fs::read(store.path())?;
        assert!(
            state
                .apply_direct_scene_edit(DirectSceneEdit::Shorten(first))
                .is_err()
        );
        assert!(
            state
                .apply_direct_scene_edit(DirectSceneEdit::Delete(first))
                .is_err()
        );
        assert_eq!(saved, fs::read(store.path())?);
        assert!(
            state
                .workspace()
                .context("workspace missing")?
                .film()
                .timeline
                .scenes[0]
                .locked
        );

        state.apply_direct_scene_edit(DirectSceneEdit::SetLocked(first, false))?;
        state
            .workspace_mut()
            .context("workspace missing")?
            .select_scene(second)?;
        state.apply_direct_scene_edit(DirectSceneEdit::Delete(second))?;
        assert_eq!(
            state
                .workspace()
                .context("workspace missing")?
                .scene_strip()
                .len(),
            1
        );
        assert_eq!(
            state
                .workspace()
                .context("workspace missing")?
                .selected_scene(),
            None
        );
        assert_eq!(
            state
                .workspace()
                .context("workspace missing")?
                .preview_plan()
                .len(),
            1
        );
        let final_disk_bytes = fs::read(store.path())?;
        assert!(
            state
                .apply_direct_scene_edit(DirectSceneEdit::Delete(first))
                .is_err()
        );
        assert_eq!(fs::read(store.path())?, final_disk_bytes);
        assert_eq!(
            store
                .load(&project)?
                .context("saved film missing")?
                .timeline
                .scenes
                .len(),
            1
        );
        assert_eq!(fs::read(temp.path().join("scene-a.jpg"))?, source_a);
        assert_eq!(fs::read(temp.path().join("scene-b.mp4"))?, source_b);
        Ok(())
    }

    #[test]
    fn stale_external_film_rejects_direct_action_before_replacing_disk_or_ui() -> Result<()> {
        let (temp, mut state) = scene_edit_fixture()?;
        let first = state
            .workspace()
            .context("workspace missing")?
            .scene_strip()[0]
            .scene_id;
        let store = FilmStore::at(temp.path().join("story"));
        let mut changed = store
            .load(state.project().context("project missing")?)?
            .context("saved film missing")?;
        changed.timeline.scenes[0].title = "Externally edited".to_owned();
        store.save(state.project().context("project missing")?, &changed)?;
        let disk_bytes = fs::read(store.path())?;
        assert!(
            state
                .apply_direct_scene_edit(DirectSceneEdit::Shorten(first))
                .is_err()
        );
        assert_eq!(disk_bytes, fs::read(store.path())?);
        assert_eq!(
            state
                .workspace()
                .context("workspace missing")?
                .scene_strip()[0]
                .title,
            "Photo"
        );
        Ok(())
    }

    #[test]
    fn reopened_project_projects_persisted_assets_into_browser_cards() -> Result<()> {
        let temp = tempdir()?;
        let image = temp.path().join("temple.jpg");
        let video = temp.path().join("street.mp4");
        fs::write(&image, b"image")?;
        fs::write(&video, b"video")?;

        let root = temp.path().join("Kyoto Trip");
        let store = ProjectStore::at(&root);
        store.create("京都之旅")?;
        store.import_media(&[image, video], &FixtureProbe)?;

        let reopened = store.open()?;
        let mut state = DesktopState::from_project(&root, reopened);
        let all = state.asset_cards();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].display_name, "temple.jpg");
        assert_eq!(all[0].detail, "1600×900");
        assert_eq!(all[1].display_name, "street.mp4");
        assert_eq!(all[1].detail, "0:13 · 1920×1080");

        state.set_filter(AssetFilter::Photos);
        assert_eq!(state.asset_cards().len(), 1);
        state.set_filter(AssetFilter::Videos);
        assert_eq!(state.asset_cards().len(), 1);
        Ok(())
    }

    #[test]
    fn desktop_state_can_bind_valid_first_cut_for_workspace_projection() -> Result<()> {
        let temp = tempdir()?;
        let image = temp.path().join("temple.jpg");
        fs::write(&image, b"image")?;
        let root = temp.path().join("Kyoto Trip");
        let store = ProjectStore::at(&root);
        store.create("京都之旅")?;
        let report = store.import_media(&[image], &FixtureProbe)?;
        let project = report.project;
        let asset_id = project.assets[0].id;
        let film = FilmState::from_draft(
            "做一个温暖的京都短片",
            FilmDraft {
                intent: IntentDraft {
                    summary: "温暖的京都短片".to_owned(),
                    target_duration_ms: Some(3_000),
                    tone_tags: vec!["温暖".to_owned()],
                },
                story: StoryDraft {
                    summary: "从寺院开始".to_owned(),
                    beats: vec![StoryBeatDraft {
                        key: "opening".to_owned(),
                        title: "开场".to_owned(),
                        purpose: "建立地点".to_owned(),
                    }],
                },
                scenes: vec![SceneDraft {
                    beat_key: "opening".to_owned(),
                    asset_id,
                    title: "寺院".to_owned(),
                    source: SceneSource::Image { duration_ms: 3_000 },
                }],
            },
            &project,
        )?;

        let state = DesktopState::from_project_with_film(&root, project, film)?;

        assert_eq!(
            state.workspace().map(|workspace| workspace.duration_ms()),
            Some(3_000)
        );
        assert_eq!(
            state
                .workspace()
                .map(|workspace| workspace.scene_strip().len()),
            Some(1)
        );
        Ok(())
    }

    #[test]
    fn normal_project_open_automatically_restores_persisted_film_workspace() -> Result<()> {
        let temp = tempdir()?;
        let image = temp.path().join("temple.jpg");
        fs::write(&image, b"image")?;
        let root = temp.path().join("Kyoto Trip");
        let store = ProjectStore::at(&root);
        store.create("京都之旅")?;
        let report = store.import_media(&[image], &FixtureProbe)?;
        let project = report.project;
        let asset_id = project.assets[0].id;
        let film = FilmState::from_draft(
            "做一个温暖的京都短片",
            FilmDraft {
                intent: IntentDraft {
                    summary: "温暖的京都短片".to_owned(),
                    target_duration_ms: Some(3_000),
                    tone_tags: vec!["温暖".to_owned()],
                },
                story: StoryDraft {
                    summary: "从寺院开始".to_owned(),
                    beats: vec![StoryBeatDraft {
                        key: "opening".to_owned(),
                        title: "开场".to_owned(),
                        purpose: "建立地点".to_owned(),
                    }],
                },
                scenes: vec![SceneDraft {
                    beat_key: "opening".to_owned(),
                    asset_id,
                    title: "寺院".to_owned(),
                    source: SceneSource::Image { duration_ms: 3_000 },
                }],
            },
            &project,
        )?;
        FilmStore::at(&root).save(&project, &film)?;

        let state = DesktopState::from_project(&root, project);

        assert!(state.error().is_none());
        assert_eq!(
            state
                .workspace()
                .map(|workspace| workspace.scene_strip().len()),
            Some(1)
        );
        assert_eq!(
            state.workspace().map(|workspace| workspace.duration_ms()),
            Some(3_000)
        );
        Ok(())
    }

    #[test]
    fn invalid_persisted_film_keeps_project_open_and_surfaces_workspace_error() -> Result<()> {
        let temp = tempdir()?;
        let root = temp.path().join("Kyoto Trip");
        let store = ProjectStore::at(&root);
        let project = store.create("京都之旅")?;
        fs::write(root.join(".laputa/film.json"), b"{ invalid film json")?;

        let state = DesktopState::from_project(&root, project);

        assert_eq!(
            state.project().map(|project| project.name.as_str()),
            Some("京都之旅")
        );
        assert!(state.workspace().is_none());
        assert!(
            state
                .error()
                .is_some_and(|error| error.contains("无法读取影片状态"))
        );
        Ok(())
    }

    #[test]
    fn failed_import_preserves_visible_inventory_and_recovers_idle_state() {
        let project = laputa_domain::ProjectManifest::new("Trip");
        let mut state = DesktopState::from_project("/tmp/laputa", project.clone());

        assert!(state.begin(DesktopOperation::ImportingMedia));
        state.fail("ffprobe unavailable");

        assert_eq!(state.project(), Some(&project));
        assert_eq!(state.operation(), DesktopOperation::Idle);
        assert_eq!(state.error(), Some("ffprobe unavailable"));
    }

    #[test]
    fn busy_state_rejects_overlapping_operations() {
        let mut state = DesktopState::launcher();
        assert!(state.begin(DesktopOperation::ChoosingProject));
        assert!(!state.begin(DesktopOperation::ChoosingMedia));
        state.cancel_operation();
        assert!(state.begin(DesktopOperation::ChoosingMedia));
    }
}
