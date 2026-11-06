//! Canonical, non-destructive revision semantics shared by future GUI and Talos entrypoints.
//!
//! A revision is validated against the existing project before `FilmStore` commits it.
//! Natural-language intent parsing and GUI pointer gestures are deliberately not part of
//! this layer: callers supply typed targets and commands.

use std::collections::HashMap;

use anyhow::{Context, Result, bail, ensure};
use laputa_domain::ProjectManifest;

use crate::{FilmState, FilmStore, SceneId, SceneSource};

/// A typed scope, not inferred from the playhead (playback and edit selection differ).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RevisionScope {
    Project,
    Scene(SceneId),
}

/// Explicit user scope wins over Selection. A stale Selection must not widen to Project.
pub fn resolve_revision_scope(
    film: &FilmState,
    selected_scene: Option<SceneId>,
    explicit_scope: Option<RevisionScope>,
) -> Result<RevisionScope> {
    let scope = explicit_scope
        .or(selected_scene.map(RevisionScope::Scene))
        .unwrap_or(RevisionScope::Project);
    if let RevisionScope::Scene(scene_id) = scope {
        ensure!(
            film.timeline
                .scenes
                .iter()
                .any(|scene| scene.id == scene_id),
            "revision target scene {scene_id:?} does not exist"
        );
    }
    Ok(scope)
}

/// Direct user action or autonomous revision. Both respect locks; only users can unlock.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RevisionOrigin {
    User,
    Automatic,
}

/// ID-based actions preserve provenance and keep imported media untouched.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RevisionAction {
    /// Move a scene immediately before another, or to the end when `before` is None.
    Reorder {
        scene_id: SceneId,
        before: Option<SceneId>,
    },
    /// Replace only the source window/duration of the target scene.
    Trim {
        scene_id: SceneId,
        source: SceneSource,
    },
    Delete {
        scene_id: SceneId,
    },
    SetLocked {
        scene_id: SceneId,
        locked: bool,
    },
}

impl FilmState {
    /// Returns a validated new FilmState; the original is never partially modified on error.
    pub fn revised(
        &self,
        project: &ProjectManifest,
        action: RevisionAction,
        origin: RevisionOrigin,
    ) -> Result<Self> {
        self.validate_for(project)
            .context("cannot revise an invalid film state")?;

        // For automatic operations, preserve both the content and ordered position of
        // retained scenes, even when a change to an unrelated scene would shift them.
        let retained: HashMap<_, _> = self
            .timeline
            .scenes
            .iter()
            .enumerate()
            .filter(|(_, scene)| scene.locked)
            .map(|(index, scene)| (scene.id, (index, scene.clone())))
            .collect();

        let mut next = self.clone();
        match action {
            RevisionAction::Reorder { scene_id, before } => {
                let original_index = scene_index(&next, scene_id)?;
                ensure_unlocked(&next, original_index)?;

                if before != Some(scene_id) {
                    let target = match before {
                        Some(target_id) => {
                            // Check existence before changing the candidate.
                            scene_index(&next, target_id)?;
                            Some(target_id)
                        }
                        None => None,
                    };
                    let scene = next.timeline.scenes.remove(original_index);
                    let new_index = match target {
                        Some(target_id) => scene_index(&next, target_id)?,
                        None => next.timeline.scenes.len(),
                    };
                    next.timeline.scenes.insert(new_index, scene);
                }
            }
            RevisionAction::Trim { scene_id, source } => {
                let index = scene_index(&next, scene_id)?;
                ensure_unlocked(&next, index)?;
                next.timeline.scenes[index].source = source;
            }
            RevisionAction::Delete { scene_id } => {
                let index = scene_index(&next, scene_id)?;
                ensure_unlocked(&next, index)?;
                ensure!(
                    next.timeline.scenes.len() > 1,
                    "cannot delete the last scene of a film"
                );
                next.timeline.scenes.remove(index);

                // FilmState requires every StoryBeat to remain represented. Drop only
                // orphaned beats; keep surviving Beat IDs and all other Scene IDs stable.
                let surviving: std::collections::HashSet<_> = next
                    .timeline
                    .scenes
                    .iter()
                    .map(|scene| scene.beat_id)
                    .collect();
                next.story.beats.retain(|beat| surviving.contains(&beat.id));
            }
            RevisionAction::SetLocked { scene_id, locked } => {
                ensure!(
                    origin == RevisionOrigin::User,
                    "automatic revision cannot change a scene lock"
                );
                let index = scene_index(&next, scene_id)?;
                next.timeline.scenes[index].locked = locked;
            }
        }

        if origin == RevisionOrigin::Automatic {
            for (scene_id, (original_index, original_scene)) in retained {
                let current_index = scene_index(&next, scene_id).with_context(|| {
                    format!("automatic revision removed retained scene {scene_id:?}")
                })?;
                ensure!(
                    current_index == original_index
                        && next.timeline.scenes[current_index] == original_scene,
                    "automatic revision would modify or move a retained scene {scene_id:?}"
                );
            }
        }

        next.validate_for(project)
            .context("revision would produce invalid film state")?;
        Ok(next)
    }
}

impl FilmStore {
    /// Apply one validated edit to the latest committed FilmState and persist only success.
    ///
    /// F1 assumes a single project writer; optimistic revision/concurrent-writer fencing
    /// belongs to a later integration slice if concurrent edits become user-reachable.
    pub fn apply_revision(
        &self,
        project: &ProjectManifest,
        action: RevisionAction,
        origin: RevisionOrigin,
    ) -> Result<FilmState> {
        let current = self
            .load(project)?
            .context("cannot edit a project without a committed first cut")?;
        let next = current.revised(project, action, origin)?;
        self.save(project, &next)?;
        Ok(next)
    }
}

fn scene_index(film: &FilmState, scene_id: SceneId) -> Result<usize> {
    film.timeline
        .scenes
        .iter()
        .position(|scene| scene.id == scene_id)
        .with_context(|| format!("scene {scene_id:?} is not in this film"))
}

fn ensure_unlocked(film: &FilmState, index: usize) -> Result<()> {
    if film.timeline.scenes[index].locked {
        bail!("scene is retained/locked; unlock it before editing");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use anyhow::Result;
    use laputa_domain::{Asset, AssetId, AssetTechnicalMetadata, MediaKind, ProjectManifest};

    use super::{RevisionAction, RevisionOrigin, RevisionScope, resolve_revision_scope};
    use crate::{
        FilmDraft, FilmState, FilmStore, IntentDraft, SceneDraft, SceneId, SceneSource,
        StoryBeatDraft, StoryDraft,
    };

    fn film_fixture(root: &std::path::Path) -> Result<(ProjectManifest, FilmState)> {
        let photo = root.join("source-photo.jpg");
        let video = root.join("source-video.mp4");
        fs::write(&photo, b"immutable photo bytes")?;
        fs::write(&video, b"immutable video bytes")?;
        let mut project = ProjectManifest::new("Test trip");
        let image_id = AssetId::new();
        let video_id = AssetId::new();
        project.assets.push(Asset {
            id: image_id,
            original_path: photo,
            display_name: "photo.jpg".into(),
            media_kind: MediaKind::Image,
            source_sha256: "photo-revision".into(),
            byte_len: 21,
            technical: AssetTechnicalMetadata {
                width: Some(640),
                height: Some(360),
                ..AssetTechnicalMetadata::default()
            },
        });
        project.assets.push(Asset {
            id: video_id,
            original_path: video,
            display_name: "video.mp4".into(),
            media_kind: MediaKind::Video,
            source_sha256: "video-revision".into(),
            byte_len: 21,
            technical: AssetTechnicalMetadata {
                width: Some(640),
                height: Some(360),
                duration_ms: Some(9_000),
                video_codec: Some("fixture".into()),
                has_audio: false,
            },
        });
        let draft = FilmDraft {
            intent: IntentDraft {
                summary: "Warm trip".into(),
                target_duration_ms: Some(7_000),
                tone_tags: vec!["warm".into()],
            },
            story: StoryDraft {
                summary: "Photo then walking".into(),
                beats: vec![
                    StoryBeatDraft {
                        key: "open".into(),
                        title: "Arrival".into(),
                        purpose: "Establish location".into(),
                    },
                    StoryBeatDraft {
                        key: "walk".into(),
                        title: "Exploring".into(),
                        purpose: "Add movement".into(),
                    },
                ],
            },
            scenes: vec![
                SceneDraft {
                    beat_key: "open".into(),
                    asset_id: image_id,
                    title: "Opening".into(),
                    source: SceneSource::Image { duration_ms: 2_000 },
                },
                SceneDraft {
                    beat_key: "walk".into(),
                    asset_id: video_id,
                    title: "Street".into(),
                    source: SceneSource::Video {
                        start_ms: 1_000,
                        end_ms: 6_000,
                    },
                },
            ],
        };
        let film = FilmState::from_draft("Make a vlog", draft, &project)?;
        Ok((project, film))
    }

    #[test]
    fn scope_selection_defaults_and_explicit_override_are_deterministic() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let (_project, film) = film_fixture(temp.path())?;
        let selected = film.timeline.scenes[1].id;

        assert_eq!(
            resolve_revision_scope(&film, None, None)?,
            RevisionScope::Project
        );
        assert_eq!(
            resolve_revision_scope(&film, Some(selected), None)?,
            RevisionScope::Scene(selected)
        );
        assert_eq!(
            resolve_revision_scope(&film, Some(selected), Some(RevisionScope::Project))?,
            RevisionScope::Project
        );
        let first = film.timeline.scenes[0].id;
        assert_eq!(
            resolve_revision_scope(&film, Some(selected), Some(RevisionScope::Scene(first)))?,
            RevisionScope::Scene(first)
        );
        assert!(resolve_revision_scope(&film, Some(SceneId::new()), None).is_err());
        assert!(
            resolve_revision_scope(
                &film,
                Some(selected),
                Some(RevisionScope::Scene(SceneId::new()))
            )
            .is_err()
        );
        Ok(())
    }

    #[test]
    fn reorder_trim_and_delete_preserve_scene_identifiers_and_validation() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let (project, film) = film_fixture(temp.path())?;
        let first = film.timeline.scenes[0].id;
        let second = film.timeline.scenes[1].id;
        let reversed = film.revised(
            &project,
            RevisionAction::Reorder {
                scene_id: second,
                before: Some(first),
            },
            RevisionOrigin::User,
        )?;
        assert_eq!(reversed.timeline.scenes[0].id, second);
        assert_eq!(reversed.timeline.scenes[1].id, first);
        assert_eq!(film.timeline.scenes[0].id, first);

        let trimmed = reversed.revised(
            &project,
            RevisionAction::Trim {
                scene_id: second,
                source: SceneSource::Video {
                    start_ms: 2_000,
                    end_ms: 4_000,
                },
            },
            RevisionOrigin::User,
        )?;
        assert_eq!(trimmed.timeline.duration_ms(), 4_000);
        assert_eq!(
            trimmed.timeline.scenes[0].asset_id,
            film.timeline.scenes[1].asset_id
        );

        let deleted = trimmed.revised(
            &project,
            RevisionAction::Delete { scene_id: first },
            RevisionOrigin::User,
        )?;
        assert_eq!(deleted.timeline.scenes.len(), 1);
        assert_eq!(deleted.story.beats.len(), 1);
        assert_eq!(deleted.timeline.scenes[0].id, second);
        assert!(
            deleted
                .revised(
                    &project,
                    RevisionAction::Delete { scene_id: second },
                    RevisionOrigin::User,
                )
                .is_err()
        );
        Ok(())
    }

    #[test]
    fn locked_scenes_block_edits_and_automatic_reordering() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let (project, film) = film_fixture(temp.path())?;
        let first = film.timeline.scenes[0].id;
        let second = film.timeline.scenes[1].id;
        let locked = film.revised(
            &project,
            RevisionAction::SetLocked {
                scene_id: first,
                locked: true,
            },
            RevisionOrigin::User,
        )?;
        assert!(locked.timeline.scenes[0].locked);

        for origin in [RevisionOrigin::User, RevisionOrigin::Automatic] {
            assert!(
                locked
                    .revised(
                        &project,
                        RevisionAction::Trim {
                            scene_id: first,
                            source: SceneSource::Image { duration_ms: 1_000 },
                        },
                        origin,
                    )
                    .is_err()
            );
            assert!(
                locked
                    .revised(&project, RevisionAction::Delete { scene_id: first }, origin,)
                    .is_err()
            );
        }
        assert!(
            locked
                .revised(
                    &project,
                    RevisionAction::Reorder {
                        scene_id: second,
                        before: Some(first),
                    },
                    RevisionOrigin::Automatic,
                )
                .is_err()
        );
        assert!(
            locked
                .revised(
                    &project,
                    RevisionAction::SetLocked {
                        scene_id: first,
                        locked: false,
                    },
                    RevisionOrigin::Automatic,
                )
                .is_err()
        );

        let unlocked = locked.revised(
            &project,
            RevisionAction::SetLocked {
                scene_id: first,
                locked: false,
            },
            RevisionOrigin::User,
        )?;
        assert!(!unlocked.timeline.scenes[0].locked);
        assert!(
            unlocked
                .revised(
                    &project,
                    RevisionAction::Trim {
                        scene_id: first,
                        source: SceneSource::Image { duration_ms: 1_000 },
                    },
                    RevisionOrigin::Automatic,
                )
                .is_ok()
        );
        Ok(())
    }

    #[test]
    fn old_schema_v1_scenes_without_locked_are_unlocked() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let (project, film) = film_fixture(temp.path())?;
        let mut value = serde_json::to_value(&film)?;
        for scene in value["timeline"]["scenes"]
            .as_array_mut()
            .expect("canonical scenes array")
        {
            scene
                .as_object_mut()
                .expect("scene object")
                .remove("locked");
        }
        let legacy: FilmState = serde_json::from_value(value)?;
        assert!(legacy.timeline.scenes.iter().all(|scene| !scene.locked));
        legacy.validate_for(&project)?;
        assert_eq!(legacy.schema_version, 1);
        Ok(())
    }

    #[test]
    fn persisted_edits_reopen_and_errors_do_not_replace_committed_film_or_originals() -> Result<()>
    {
        let temp = tempfile::tempdir()?;
        let (project, film) = film_fixture(temp.path())?;
        let store = FilmStore::at(temp.path());
        store.save(&project, &film)?;
        let first = film.timeline.scenes[0].id;
        let second = film.timeline.scenes[1].id;
        let originals: Vec<_> = project
            .assets
            .iter()
            .map(|asset| fs::read(&asset.original_path))
            .collect::<std::io::Result<_>>()?;
        let updated = store.apply_revision(
            &project,
            RevisionAction::Reorder {
                scene_id: second,
                before: Some(first),
            },
            RevisionOrigin::User,
        )?;
        assert_eq!(store.load(&project)?, Some(updated.clone()));

        let locked = store.apply_revision(
            &project,
            RevisionAction::SetLocked {
                scene_id: second,
                locked: true,
            },
            RevisionOrigin::User,
        )?;
        assert_eq!(locked.timeline.scenes[0].id, second);
        assert!(locked.timeline.scenes[0].locked);
        let committed_bytes = fs::read(store.path())?;

        assert!(
            store
                .apply_revision(
                    &project,
                    RevisionAction::Trim {
                        scene_id: first,
                        source: SceneSource::Image { duration_ms: 0 },
                    },
                    RevisionOrigin::User,
                )
                .is_err()
        );
        assert!(
            store
                .apply_revision(
                    &project,
                    RevisionAction::Delete {
                        scene_id: SceneId::new(),
                    },
                    RevisionOrigin::User,
                )
                .is_err()
        );
        assert_eq!(fs::read(store.path())?, committed_bytes);
        assert_eq!(store.load(&project)?, Some(locked));

        for (asset, original) in project.assets.iter().zip(originals) {
            assert_eq!(fs::read(&asset.original_path)?, original);
        }
        assert_eq!(
            project.assets[0].original_path,
            temp.path().join("source-photo.jpg")
        );
        Ok(())
    }
}
