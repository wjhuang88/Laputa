use async_trait::async_trait;
use laputa_domain::{AssetAnalysis, AssetAnalysisState, AssetId, MediaKind};
use laputa_media::ProjectStore;
use laputa_timeline::{FilmDraft, FilmState, FilmStore};
use serde::Serialize;
use serde_json::{Value, json};
use talos_runtime::{AgentTool, ToolNature, ToolResult};

pub const PROJECT_INSPECT_TOOL: &str = "project.inspect";
pub const FILM_COMMIT_FIRST_CUT_TOOL: &str = "film.commit_first_cut";

#[derive(Clone)]
pub struct ProjectInspectTool {
    store: ProjectStore,
}

impl ProjectInspectTool {
    #[must_use]
    pub fn new(store: ProjectStore) -> Self {
        Self { store }
    }
}

#[derive(Serialize)]
struct ProjectSnapshot {
    name: String,
    assets: Vec<AssetSnapshot>,
}

#[derive(Serialize)]
struct AssetSnapshot {
    id: AssetId,
    display_name: String,
    media_kind: MediaKind,
    duration_ms: Option<u64>,
    analysis_status: &'static str,
    analysis: Option<AssetAnalysis>,
}

#[async_trait]
impl AgentTool for ProjectInspectTool {
    fn name(&self) -> &str {
        PROJECT_INSPECT_TOOL
    }

    fn description(&self) -> &str {
        "Inspect imported project moments and their canonical Laputa analysis for story planning"
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {},
            "additionalProperties": false
        })
    }

    async fn execute(&self, _input: Value) -> ToolResult {
        match self.snapshot() {
            Ok(snapshot) => match serde_json::to_string(&snapshot) {
                Ok(content) => ToolResult::success(content),
                Err(error) => ToolResult::error(format!("failed to serialize project snapshot: {error}")),
            },
            Err(error) => ToolResult::error(error.to_string()),
        }
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn nature(&self) -> ToolNature {
        ToolNature::Read
    }
}

impl ProjectInspectTool {
    fn snapshot(&self) -> anyhow::Result<ProjectSnapshot> {
        let project = self.store.open()?;
        let mut assets = Vec::with_capacity(project.assets.len());
        for asset in project.assets {
            let state = self.store.load_understanding_state(&asset)?;
            let (analysis_status, analysis) = match state {
                Some(AssetAnalysisState::Ready(record)) => ("ready", Some(record.analysis)),
                Some(AssetAnalysisState::Failed(_)) => ("failed", None),
                None => ("missing", None),
            };
            assets.push(AssetSnapshot {
                id: asset.id,
                display_name: asset.display_name,
                media_kind: asset.media_kind,
                duration_ms: asset.technical.duration_ms,
                analysis_status,
                analysis,
            });
        }
        Ok(ProjectSnapshot {
            name: project.name,
            assets,
        })
    }
}

#[derive(Clone)]
pub struct CommitFirstCutTool {
    project_store: ProjectStore,
    film_store: FilmStore,
    user_request: String,
}

impl CommitFirstCutTool {
    #[must_use]
    pub fn new(project_store: ProjectStore, user_request: impl Into<String>) -> Self {
        let film_store = FilmStore::at(project_store.root());
        Self {
            project_store,
            film_store,
            user_request: user_request.into(),
        }
    }
}

#[async_trait]
impl AgentTool for CommitFirstCutTool {
    fn name(&self) -> &str {
        FILM_COMMIT_FIRST_CUT_TOOL
    }

    fn description(&self) -> &str {
        "Atomically commit one validated Laputa first cut containing intent, story beats, and ordered scenes"
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "intent": {
                    "type": "object",
                    "properties": {
                        "summary": {"type": "string"},
                        "target_duration_ms": {"type": ["integer", "null"], "minimum": 1},
                        "tone_tags": {"type": "array", "items": {"type": "string"}}
                    },
                    "required": ["summary", "target_duration_ms", "tone_tags"],
                    "additionalProperties": false
                },
                "story": {
                    "type": "object",
                    "properties": {
                        "summary": {"type": "string"},
                        "beats": {
                            "type": "array",
                            "minItems": 1,
                            "items": {
                                "type": "object",
                                "properties": {
                                    "key": {"type": "string"},
                                    "title": {"type": "string"},
                                    "purpose": {"type": "string"}
                                },
                                "required": ["key", "title", "purpose"],
                                "additionalProperties": false
                            }
                        }
                    },
                    "required": ["summary", "beats"],
                    "additionalProperties": false
                },
                "scenes": {
                    "type": "array",
                    "minItems": 1,
                    "items": {
                        "type": "object",
                        "properties": {
                            "beat_key": {"type": "string"},
                            "asset_id": {"type": "string", "format": "uuid"},
                            "title": {"type": "string"},
                            "source": {
                                "oneOf": [
                                    {
                                        "type": "object",
                                        "properties": {
                                            "kind": {"const": "image"},
                                            "duration_ms": {"type": "integer", "minimum": 1}
                                        },
                                        "required": ["kind", "duration_ms"],
                                        "additionalProperties": false
                                    },
                                    {
                                        "type": "object",
                                        "properties": {
                                            "kind": {"const": "video"},
                                            "start_ms": {"type": "integer", "minimum": 0},
                                            "end_ms": {"type": "integer", "minimum": 1}
                                        },
                                        "required": ["kind", "start_ms", "end_ms"],
                                        "additionalProperties": false
                                    }
                                ]
                            }
                        },
                        "required": ["beat_key", "asset_id", "title", "source"],
                        "additionalProperties": false
                    }
                }
            },
            "required": ["intent", "story", "scenes"],
            "additionalProperties": false
        })
    }

    async fn execute(&self, input: Value) -> ToolResult {
        match self.commit(input) {
            Ok(state) => ToolResult::success(
                json!({
                    "status": "committed",
                    "story_summary": state.story.summary,
                    "scene_count": state.timeline.scenes.len(),
                    "duration_ms": state.timeline.duration_ms()
                })
                .to_string(),
            ),
            Err(error) => ToolResult::error(error.to_string()),
        }
    }

    fn nature(&self) -> ToolNature {
        ToolNature::Write
    }
}

impl CommitFirstCutTool {
    fn commit(&self, input: Value) -> anyhow::Result<FilmState> {
        let draft: FilmDraft = serde_json::from_value(input)
            .map_err(|error| anyhow::anyhow!("invalid first-cut draft: {error}"))?;
        let project = self.project_store.open()?;
        let state = FilmState::from_draft(self.user_request.clone(), draft, &project)?;
        self.film_store.save(&project, &state)?;
        Ok(state)
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use anyhow::Result;
    use laputa_domain::{AssetAnalysis, AssetTechnicalMetadata, MediaKind};
    use laputa_media::{MediaProbe, ProjectStore};
    use laputa_timeline::{
        FilmDraft, FilmStore, IntentDraft, SceneDraft, SceneSource, StoryBeatDraft, StoryDraft,
    };
    use talos_runtime::AgentTool;

    use super::{CommitFirstCutTool, ProjectInspectTool};

    #[derive(Debug)]
    struct FixtureProbe;

    impl MediaProbe for FixtureProbe {
        fn probe(&self, _source: &Path, kind: MediaKind) -> Result<AssetTechnicalMetadata> {
            Ok(match kind {
                MediaKind::Image => AssetTechnicalMetadata {
                    width: Some(1920),
                    height: Some(1080),
                    ..AssetTechnicalMetadata::default()
                },
                MediaKind::Video => AssetTechnicalMetadata {
                    width: Some(1920),
                    height: Some(1080),
                    duration_ms: Some(8_000),
                    video_codec: Some("fixture".to_owned()),
                    has_audio: true,
                },
            })
        }
    }

    fn analysis(summary: &str, video: bool) -> AssetAnalysis {
        AssetAnalysis {
            summary: summary.to_owned(),
            tags: vec!["travel".to_owned()],
            segments: if video {
                vec![laputa_domain::AnalysisSegment {
                    range: laputa_domain::AnalysisTimeRange {
                        start_ms: 0,
                        end_ms: 8_000,
                    },
                    summary: "Walking through the street".to_owned(),
                    tags: vec!["walking".to_owned()],
                }]
            } else {
                Vec::new()
            },
        }
    }

    fn prepared_project() -> Result<(tempfile::TempDir, ProjectStore)> {
        let temp = tempfile::tempdir()?;
        let image = temp.path().join("photo.jpg");
        let video = temp.path().join("clip.mp4");
        fs::write(&image, b"fixture-image")?;
        fs::write(&video, b"fixture-video")?;
        let store = ProjectStore::at(temp.path().join("project"));
        store.create("Kyoto Trip")?;
        let report = store.import_media(&[image, video], &FixtureProbe)?;
        store.record_understanding_ready(
            &report.imported[0],
            &report.imported[0].source_sha256,
            analysis("Temple portrait", false),
        )?;
        store.record_understanding_ready(
            &report.imported[1],
            &report.imported[1].source_sha256,
            analysis("Kyoto street", true),
        )?;
        Ok((temp, store))
    }

    fn draft(store: &ProjectStore) -> Result<FilmDraft> {
        let project = store.open()?;
        Ok(FilmDraft {
            intent: IntentDraft {
                summary: "Warm one-minute Kyoto memory".to_owned(),
                target_duration_ms: Some(60_000),
                tone_tags: vec!["warm".to_owned(), "natural".to_owned()],
            },
            story: StoryDraft {
                summary: "Quiet arrival followed by movement".to_owned(),
                beats: vec![
                    StoryBeatDraft {
                        key: "arrival".to_owned(),
                        title: "Arrival".to_owned(),
                        purpose: "Establish Kyoto".to_owned(),
                    },
                    StoryBeatDraft {
                        key: "walk".to_owned(),
                        title: "Explore".to_owned(),
                        purpose: "Move into the street".to_owned(),
                    },
                ],
            },
            scenes: vec![
                SceneDraft {
                    beat_key: "arrival".to_owned(),
                    asset_id: project.assets[0].id,
                    title: "Temple portrait".to_owned(),
                    source: SceneSource::Image { duration_ms: 4_000 },
                },
                SceneDraft {
                    beat_key: "walk".to_owned(),
                    asset_id: project.assets[1].id,
                    title: "Street walk".to_owned(),
                    source: SceneSource::Video {
                        start_ms: 1_000,
                        end_ms: 7_000,
                    },
                },
            ],
        })
    }

    #[tokio::test]
    async fn inspect_exposes_canonical_ready_analysis() -> Result<()> {
        let (_temp, store) = prepared_project()?;
        let result = ProjectInspectTool::new(store)
            .execute(serde_json::json!({}))
            .await;

        assert!(!result.is_error);
        let value: serde_json::Value = serde_json::from_str(&result.content)?;
        assert_eq!(value["name"], "Kyoto Trip");
        assert_eq!(value["assets"].as_array().map(Vec::len), Some(2));
        assert_eq!(value["assets"][0]["analysis_status"], "ready");
        assert_eq!(value["assets"][0]["analysis"]["summary"], "Temple portrait");
        Ok(())
    }

    #[tokio::test]
    async fn commit_tool_persists_valid_first_cut() -> Result<()> {
        let (_temp, store) = prepared_project()?;
        let tool = CommitFirstCutTool::new(store.clone(), "Make a warm one-minute Kyoto vlog");
        let result = tool.execute(serde_json::to_value(draft(&store)?)?).await;

        assert!(!result.is_error);
        let project = store.open()?;
        let persisted = FilmStore::at(store.root())
            .load(&project)?
            .expect("film state should be persisted");
        assert_eq!(persisted.intent.request, "Make a warm one-minute Kyoto vlog");
        assert_eq!(persisted.timeline.scenes.len(), 2);
        Ok(())
    }

    #[tokio::test]
    async fn rejected_commit_does_not_create_film_state() -> Result<()> {
        let (_temp, store) = prepared_project()?;
        let mut candidate = draft(&store)?;
        candidate.scenes[0].asset_id = laputa_domain::AssetId::new();
        let tool = CommitFirstCutTool::new(store.clone(), "Make a vlog");

        let result = tool.execute(serde_json::to_value(candidate)?).await;

        assert!(result.is_error);
        assert!(!FilmStore::at(store.root()).path().exists());
        Ok(())
    }
}
