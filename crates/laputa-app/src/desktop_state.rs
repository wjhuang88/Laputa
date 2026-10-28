use std::path::{Path, PathBuf};

use anyhow::Result;
use laputa_domain::{Asset, AssetId, MediaKind, ProjectManifest};
use laputa_timeline::FilmState;

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
        Self {
            project_root: Some(root.into()),
            project: Some(project),
            ..Self::default()
        }
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
        self.project_root = Some(root);
        self.project = Some(project);
        self.workspace = None;
        self.filter = AssetFilter::All;
        self.operation = DesktopOperation::Idle;
        self.error = None;
        self.notice = None;
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

    use anyhow::Result;
    use laputa_domain::{AssetTechnicalMetadata, MediaKind};
    use laputa_media::{MediaProbe, ProjectStore};
    use laputa_timeline::{
        FilmDraft, FilmState, IntentDraft, SceneDraft, SceneSource, StoryBeatDraft, StoryDraft,
    };
    use tempfile::tempdir;

    use super::{AssetFilter, DesktopOperation, DesktopState};

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
