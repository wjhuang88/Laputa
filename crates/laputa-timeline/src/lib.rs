//! Laputa-owned editorial state and timeline intermediate representation.

use std::{
    collections::{HashMap, HashSet},
    fmt,
    fs::{self, File},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result as AnyResult};
use laputa_domain::{AssetId, MediaKind, ProjectManifest};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const FILM_SCHEMA_VERSION: u32 = 1;
const STATE_DIRECTORY: &str = ".laputa";
const FILM_STATE_FILE: &str = "film.json";

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct StoryBeatId(Uuid);

impl StoryBeatId {
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for StoryBeatId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SceneId(Uuid);

impl SceneId {
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for SceneId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct IntentSpec {
    pub request: String,
    pub summary: String,
    pub target_duration_ms: Option<u64>,
    #[serde(default)]
    pub tone_tags: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct StoryPlan {
    pub summary: String,
    pub beats: Vec<StoryBeat>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct StoryBeat {
    pub id: StoryBeatId,
    pub title: String,
    pub purpose: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Timeline {
    pub scenes: Vec<Scene>,
}

impl Timeline {
    #[must_use]
    pub fn duration_ms(&self) -> u64 {
        self.scenes.iter().fold(0_u64, |total, scene| {
            total.saturating_add(scene.source.duration_ms())
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Scene {
    pub id: SceneId,
    pub beat_id: StoryBeatId,
    pub asset_id: AssetId,
    pub title: String,
    pub source: SceneSource,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SceneSource {
    Image { duration_ms: u64 },
    Video { start_ms: u64, end_ms: u64 },
}

impl SceneSource {
    #[must_use]
    pub fn duration_ms(&self) -> u64 {
        match self {
            Self::Image { duration_ms } => *duration_ms,
            Self::Video { start_ms, end_ms } => end_ms.saturating_sub(*start_ms),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FilmState {
    pub schema_version: u32,
    pub intent: IntentSpec,
    pub story: StoryPlan,
    pub timeline: Timeline,
}

impl FilmState {
    pub fn from_draft(
        request: impl Into<String>,
        draft: FilmDraft,
        project: &ProjectManifest,
    ) -> Result<Self, FilmValidationError> {
        let request = request.into();
        let mut beat_ids = HashMap::with_capacity(draft.story.beats.len());
        let mut beats = Vec::with_capacity(draft.story.beats.len());

        for beat in draft.story.beats {
            ensure_text("story beat key", &beat.key)?;
            if beat_ids.contains_key(&beat.key) {
                return Err(FilmValidationError::new(format!(
                    "story beat key '{}' is duplicated",
                    beat.key
                )));
            }
            let id = StoryBeatId::new();
            beat_ids.insert(beat.key, id);
            beats.push(StoryBeat {
                id,
                title: beat.title,
                purpose: beat.purpose,
            });
        }

        let mut scenes = Vec::with_capacity(draft.scenes.len());
        for scene in draft.scenes {
            let beat_id = beat_ids.get(&scene.beat_key).copied().ok_or_else(|| {
                FilmValidationError::new(format!(
                    "scene '{}' references unknown story beat key '{}'",
                    scene.title, scene.beat_key
                ))
            })?;
            scenes.push(Scene {
                id: SceneId::new(),
                beat_id,
                asset_id: scene.asset_id,
                title: scene.title,
                source: scene.source,
            });
        }

        let state = Self {
            schema_version: FILM_SCHEMA_VERSION,
            intent: IntentSpec {
                request,
                summary: draft.intent.summary,
                target_duration_ms: draft.intent.target_duration_ms,
                tone_tags: draft.intent.tone_tags,
            },
            story: StoryPlan {
                summary: draft.story.summary,
                beats,
            },
            timeline: Timeline { scenes },
        };
        state.validate_for(project)?;
        Ok(state)
    }

    pub fn validate_for(&self, project: &ProjectManifest) -> Result<(), FilmValidationError> {
        if self.schema_version != FILM_SCHEMA_VERSION {
            return Err(FilmValidationError::new(format!(
                "unsupported film schema {}; expected {}",
                self.schema_version, FILM_SCHEMA_VERSION
            )));
        }
        ensure_text("intent request", &self.intent.request)?;
        ensure_text("intent summary", &self.intent.summary)?;
        if self.intent.target_duration_ms == Some(0) {
            return Err(FilmValidationError::new(
                "target duration must be greater than zero when provided",
            ));
        }
        ensure_non_empty_tags("intent tone tags", &self.intent.tone_tags)?;
        ensure_text("story summary", &self.story.summary)?;
        if self.story.beats.is_empty() {
            return Err(FilmValidationError::new(
                "story must contain at least one beat",
            ));
        }
        if self.timeline.scenes.is_empty() {
            return Err(FilmValidationError::new(
                "timeline must contain at least one scene",
            ));
        }

        let mut beat_ids = HashSet::with_capacity(self.story.beats.len());
        for (index, beat) in self.story.beats.iter().enumerate() {
            if !beat_ids.insert(beat.id) {
                return Err(FilmValidationError::new(format!(
                    "story beat {index} has a duplicate id"
                )));
            }
            ensure_text("story beat title", &beat.title)?;
            ensure_text("story beat purpose", &beat.purpose)?;
        }

        let mut scene_ids = HashSet::with_capacity(self.timeline.scenes.len());
        let mut used_beats = HashSet::with_capacity(self.story.beats.len());
        for (index, scene) in self.timeline.scenes.iter().enumerate() {
            if !scene_ids.insert(scene.id) {
                return Err(FilmValidationError::new(format!(
                    "scene {index} has a duplicate id"
                )));
            }
            if !beat_ids.contains(&scene.beat_id) {
                return Err(FilmValidationError::new(format!(
                    "scene {index} references a story beat that does not exist"
                )));
            }
            used_beats.insert(scene.beat_id);
            ensure_text("scene title", &scene.title)?;
            let asset = project.asset(scene.asset_id).ok_or_else(|| {
                FilmValidationError::new(format!(
                    "scene {index} references asset {} that is not in the project",
                    scene.asset_id
                ))
            })?;
            validate_scene_source(
                index,
                &scene.source,
                asset.media_kind,
                asset.technical.duration_ms,
            )?;
        }

        if used_beats.len() != beat_ids.len() {
            return Err(FilmValidationError::new(
                "every story beat must be represented by at least one scene",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FilmDraft {
    pub intent: IntentDraft,
    pub story: StoryDraft,
    pub scenes: Vec<SceneDraft>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct IntentDraft {
    pub summary: String,
    pub target_duration_ms: Option<u64>,
    #[serde(default)]
    pub tone_tags: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct StoryDraft {
    pub summary: String,
    pub beats: Vec<StoryBeatDraft>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct StoryBeatDraft {
    pub key: String,
    pub title: String,
    pub purpose: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SceneDraft {
    pub beat_key: String,
    pub asset_id: AssetId,
    pub title: String,
    pub source: SceneSource,
}

#[derive(Clone, Debug)]
pub struct FilmStore {
    project_root: PathBuf,
}

impl FilmStore {
    #[must_use]
    pub fn at(project_root: impl Into<PathBuf>) -> Self {
        Self {
            project_root: project_root.into(),
        }
    }

    #[must_use]
    pub fn path(&self) -> PathBuf {
        self.project_root.join(STATE_DIRECTORY).join(FILM_STATE_FILE)
    }

    pub fn load(&self, project: &ProjectManifest) -> AnyResult<Option<FilmState>> {
        let path = self.path();
        if !path.exists() {
            return Ok(None);
        }
        let bytes = fs::read(&path)
            .with_context(|| format!("failed to read film state {}", path.display()))?;
        let state: FilmState = serde_json::from_slice(&bytes)
            .with_context(|| format!("invalid film state {}", path.display()))?;
        state
            .validate_for(project)
            .context("persisted film state is invalid for the current project")?;
        Ok(Some(state))
    }

    pub fn save(&self, project: &ProjectManifest, state: &FilmState) -> AnyResult<()> {
        state
            .validate_for(project)
            .context("cannot persist invalid film state")?;
        let path = self.path();
        let parent = path
            .parent()
            .context("film state path has no parent directory")?;
        fs::create_dir_all(parent).with_context(|| {
            format!("failed to create film state directory {}", parent.display())
        })?;
        let bytes = serde_json::to_vec_pretty(state).context("failed to serialize film state")?;
        let temporary = parent.join(format!(
            ".{FILM_STATE_FILE}.tmp-{}",
            std::process::id()
        ));
        fs::write(&temporary, bytes)
            .with_context(|| format!("failed to write temporary film state {}", temporary.display()))?;
        File::open(&temporary)
            .and_then(|file| file.sync_all())
            .with_context(|| format!("failed to sync temporary film state {}", temporary.display()))?;
        install_replacement(&temporary, &path)
    }
}

fn install_replacement(temporary: &Path, destination: &Path) -> AnyResult<()> {
    match fs::rename(temporary, destination) {
        Ok(()) => Ok(()),
        Err(error) if destination.exists() => {
            fs::remove_file(destination).with_context(|| {
                format!(
                    "failed to replace film state after rename error: {error}"
                )
            })?;
            fs::rename(temporary, destination).context("failed to install replacement film state")
        }
        Err(error) => Err(error).context("failed to install film state"),
    }
}

fn validate_scene_source(
    index: usize,
    source: &SceneSource,
    media_kind: MediaKind,
    source_duration_ms: Option<u64>,
) -> Result<(), FilmValidationError> {
    match (media_kind, source) {
        (MediaKind::Image, SceneSource::Image { duration_ms }) => {
            if *duration_ms == 0 {
                return Err(FilmValidationError::new(format!(
                    "image scene {index} must have positive duration"
                )));
            }
        }
        (MediaKind::Video, SceneSource::Video { start_ms, end_ms }) => {
            if start_ms >= end_ms {
                return Err(FilmValidationError::new(format!(
                    "video scene {index} has an empty or reversed source range"
                )));
            }
            if let Some(duration_ms) = source_duration_ms
                && *end_ms > duration_ms
            {
                return Err(FilmValidationError::new(format!(
                    "video scene {index} ends after the source duration"
                )));
            }
        }
        (MediaKind::Image, SceneSource::Video { .. }) => {
            return Err(FilmValidationError::new(format!(
                "image asset in scene {index} cannot use a video source range"
            )));
        }
        (MediaKind::Video, SceneSource::Image { .. }) => {
            return Err(FilmValidationError::new(format!(
                "video asset in scene {index} cannot use image duration semantics"
            )));
        }
    }
    Ok(())
}

fn ensure_text(field: &str, value: &str) -> Result<(), FilmValidationError> {
    if value.trim().is_empty() {
        return Err(FilmValidationError::new(format!(
            "{field} must not be empty"
        )));
    }
    Ok(())
}

fn ensure_non_empty_tags(field: &str, tags: &[String]) -> Result<(), FilmValidationError> {
    if tags.iter().any(|tag| tag.trim().is_empty()) {
        return Err(FilmValidationError::new(format!(
            "{field} must not contain empty values"
        )));
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FilmValidationError {
    message: String,
}

impl FilmValidationError {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for FilmValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for FilmValidationError {}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use laputa_domain::{Asset, AssetId, AssetTechnicalMetadata, MediaKind, ProjectManifest};

    use super::{
        FilmDraft, FilmState, FilmStore, IntentDraft, SceneDraft, SceneSource, StoryBeatDraft,
        StoryDraft,
    };

    fn project() -> ProjectManifest {
        let mut project = ProjectManifest::new("Trip");
        project.assets.push(Asset {
            id: AssetId::new(),
            original_path: PathBuf::from("photo.jpg"),
            display_name: "photo.jpg".to_owned(),
            media_kind: MediaKind::Image,
            source_sha256: "image-revision".to_owned(),
            byte_len: 1,
            technical: AssetTechnicalMetadata {
                width: Some(1920),
                height: Some(1080),
                ..AssetTechnicalMetadata::default()
            },
        });
        project.assets.push(Asset {
            id: AssetId::new(),
            original_path: PathBuf::from("clip.mp4"),
            display_name: "clip.mp4".to_owned(),
            media_kind: MediaKind::Video,
            source_sha256: "video-revision".to_owned(),
            byte_len: 1,
            technical: AssetTechnicalMetadata {
                width: Some(1920),
                height: Some(1080),
                duration_ms: Some(8_000),
                video_codec: Some("fixture".to_owned()),
                has_audio: true,
            },
        });
        project
    }

    fn valid_draft(project: &ProjectManifest) -> FilmDraft {
        FilmDraft {
            intent: IntentDraft {
                summary: "A warm short travel film".to_owned(),
                target_duration_ms: Some(10_000),
                tone_tags: vec!["warm".to_owned()],
            },
            story: StoryDraft {
                summary: "Arrival then movement through the city".to_owned(),
                beats: vec![
                    StoryBeatDraft {
                        key: "open".to_owned(),
                        title: "Arrival".to_owned(),
                        purpose: "Establish the place".to_owned(),
                    },
                    StoryBeatDraft {
                        key: "move".to_owned(),
                        title: "Explore".to_owned(),
                        purpose: "Show motion and discovery".to_owned(),
                    },
                ],
            },
            scenes: vec![
                SceneDraft {
                    beat_key: "open".to_owned(),
                    asset_id: project.assets[0].id,
                    title: "Opening photo".to_owned(),
                    source: SceneSource::Image { duration_ms: 3_000 },
                },
                SceneDraft {
                    beat_key: "move".to_owned(),
                    asset_id: project.assets[1].id,
                    title: "Walking clip".to_owned(),
                    source: SceneSource::Video {
                        start_ms: 1_000,
                        end_ms: 6_000,
                    },
                },
            ],
        }
    }

    #[test]
    fn valid_draft_becomes_canonical_film_state() {
        let project = project();
        let state = FilmState::from_draft(
            "Make a warm short trip vlog",
            valid_draft(&project),
            &project,
        )
        .expect("valid draft should become film state");

        assert_eq!(state.intent.request, "Make a warm short trip vlog");
        assert_eq!(state.story.beats.len(), 2);
        assert_eq!(state.timeline.scenes.len(), 2);
        assert_eq!(state.timeline.duration_ms(), 8_000);
        assert_ne!(state.story.beats[0].id, state.story.beats[1].id);
        assert_ne!(state.timeline.scenes[0].id, state.timeline.scenes[1].id);
    }

    #[test]
    fn unknown_asset_is_rejected() {
        let project = project();
        let mut draft = valid_draft(&project);
        draft.scenes[0].asset_id = AssetId::new();

        let error = FilmState::from_draft("Trip", draft, &project)
            .expect_err("unknown asset must be rejected");
        assert!(error.to_string().contains("not in the project"));
    }

    #[test]
    fn video_range_past_source_duration_is_rejected() {
        let project = project();
        let mut draft = valid_draft(&project);
        draft.scenes[1].source = SceneSource::Video {
            start_ms: 7_000,
            end_ms: 9_000,
        };

        let error = FilmState::from_draft("Trip", draft, &project)
            .expect_err("out-of-bounds video range must be rejected");
        assert!(error.to_string().contains("source duration"));
    }

    #[test]
    fn every_story_beat_requires_a_scene() {
        let project = project();
        let mut draft = valid_draft(&project);
        draft.scenes.pop();

        let error = FilmState::from_draft("Trip", draft, &project)
            .expect_err("unused story beat must be rejected");
        assert!(error.to_string().contains("every story beat"));
    }

    #[test]
    fn film_store_round_trips_without_touching_project_manifest_file() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let project = project();
        let state = FilmState::from_draft("Trip", valid_draft(&project), &project)?;
        let state_dir = temp.path().join(".laputa");
        fs::create_dir_all(&state_dir)?;
        let manifest_path = state_dir.join("project.json");
        fs::write(&manifest_path, b"story-a-manifest-sentinel")?;
        let store = FilmStore::at(temp.path());

        store.save(&project, &state)?;
        let reopened = store.load(&project)?.expect("film state should reopen");

        assert_eq!(reopened, state);
        assert_eq!(fs::read(&manifest_path)?, b"story-a-manifest-sentinel");
        Ok(())
    }

    #[test]
    fn invalid_state_does_not_replace_valid_persisted_state() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let project = project();
        let valid = FilmState::from_draft("Trip", valid_draft(&project), &project)?;
        let store = FilmStore::at(temp.path());
        store.save(&project, &valid)?;

        let mut invalid = valid.clone();
        invalid.intent.summary.clear();
        assert!(store.save(&project, &invalid).is_err());
        assert_eq!(store.load(&project)?, Some(valid));
        Ok(())
    }
}
