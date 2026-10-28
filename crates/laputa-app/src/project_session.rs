use std::path::Path;

use anyhow::{Context, Result};
use laputa_media::ProjectStore;
use laputa_timeline::FilmStore;

use crate::desktop_state::DesktopState;

pub fn open_project_state(root: impl AsRef<Path>) -> Result<DesktopState> {
    let root = root.as_ref();
    let project = ProjectStore::at(root)
        .open()
        .with_context(|| format!("failed to open Laputa project {}", root.display()))?;
    let film = FilmStore::at(root)
        .load(&project)
        .with_context(|| format!("failed to open FilmState for {}", root.display()))?;

    match film {
        Some(film) => DesktopState::from_project_with_film(root, project, film),
        None => Ok(DesktopState::from_project(root, project)),
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use anyhow::Result;
    use laputa_domain::{AssetTechnicalMetadata, MediaKind};
    use laputa_media::{MediaProbe, ProjectStore};
    use laputa_timeline::{
        FilmDraft, FilmState, FilmStore, IntentDraft, SceneDraft, SceneSource, StoryBeatDraft,
        StoryDraft,
    };
    use tempfile::tempdir;

    use super::open_project_state;

    struct FixtureProbe;

    impl MediaProbe for FixtureProbe {
        fn probe(&self, _source: &Path, _kind: MediaKind) -> Result<AssetTechnicalMetadata> {
            Ok(AssetTechnicalMetadata {
                width: Some(1600),
                height: Some(900),
                ..AssetTechnicalMetadata::default()
            })
        }
    }

    #[test]
    fn reopen_without_film_keeps_asset_workspace_available() -> Result<()> {
        let temp = tempdir()?;
        let source = temp.path().join("photo.jpg");
        fs::write(&source, b"photo")?;
        let root = temp.path().join("project");
        let store = ProjectStore::at(&root);
        store.create("Trip")?;
        store.import_media(&[source], &FixtureProbe)?;

        let reopened = open_project_state(&root)?;

        assert_eq!(reopened.asset_count(), 1);
        assert!(reopened.workspace().is_none());
        Ok(())
    }

    #[test]
    fn reopen_with_film_restores_scene_strip_projection() -> Result<()> {
        let temp = tempdir()?;
        let source = temp.path().join("photo.jpg");
        fs::write(&source, b"photo")?;
        let root = temp.path().join("project");
        let store = ProjectStore::at(&root);
        store.create("Trip")?;
        let report = store.import_media(&[source], &FixtureProbe)?;
        let project = report.project;
        let asset_id = project.assets[0].id;
        let film = FilmState::from_draft(
            "Make a quiet trip film",
            FilmDraft {
                intent: IntentDraft {
                    summary: "Quiet trip film".to_owned(),
                    target_duration_ms: Some(4_000),
                    tone_tags: vec!["quiet".to_owned()],
                },
                story: StoryDraft {
                    summary: "A single opening moment".to_owned(),
                    beats: vec![StoryBeatDraft {
                        key: "opening".to_owned(),
                        title: "Opening".to_owned(),
                        purpose: "Establish the memory".to_owned(),
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
        FilmStore::at(&root).save(&project, &film)?;

        let reopened = open_project_state(&root)?;
        let workspace = reopened
            .workspace()
            .expect("persisted FilmState should restore workspace projection");

        assert_eq!(workspace.scene_strip().len(), 1);
        assert_eq!(workspace.scene_strip()[0].title, "Opening photo");
        assert_eq!(workspace.duration_ms(), 4_000);
        Ok(())
    }
}
