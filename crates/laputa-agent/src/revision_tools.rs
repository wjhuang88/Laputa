//! Laputa-owned, constrained Talos tools for editing an existing film.
//!
//! Tools only stage a candidate. A successful Talos turn is required before the
//! runner writes one FilmState; a failed turn never leaves a partial edit behind.

use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

use anyhow::{Context, Result, bail, ensure};
use async_trait::async_trait;
use laputa_domain::ProjectManifest;
use laputa_timeline::{
    FilmState, SceneId, SceneSource,
    revision::{RevisionAction, RevisionOrigin, RevisionScope, resolve_revision_scope},
};
use serde::Deserialize;
use serde_json::{Value, json};
use talos_runtime::{AgentTool, ToolNature, ToolResult};

pub const TIMELINE_GET_TOOL: &str = "timeline.get";
pub const TIMELINE_EDIT_TOOL: &str = "timeline.edit";

#[derive(Default)]
pub(crate) struct RevisionTurnState {
    pub prepared: Mutex<Option<FilmState>>,
    pub rejected: AtomicBool,
    pub inspected: AtomicBool,
}

#[derive(Clone)]
pub(crate) struct RevisionTurn {
    pub project: ProjectManifest,
    pub film: FilmState,
    pub selected_scene: Option<SceneId>,
    pub request: String,
    pub state: Arc<RevisionTurnState>,
}

impl RevisionTurn {
    pub fn new(
        project: ProjectManifest,
        film: FilmState,
        selected_scene: Option<SceneId>,
        request: String,
    ) -> Self {
        Self {
            project,
            film,
            selected_scene,
            request,
            state: Arc::new(RevisionTurnState::default()),
        }
    }
}

#[derive(Clone)]
pub struct TimelineGetTool {
    turn: RevisionTurn,
}

impl TimelineGetTool {
    pub(crate) fn new(turn: RevisionTurn) -> Self {
        Self { turn }
    }
}

#[async_trait]
impl AgentTool for TimelineGetTool {
    fn name(&self) -> &str {
        TIMELINE_GET_TOOL
    }

    fn description(&self) -> &str {
        "Inspect the current film scenes, their source windows, retain locks and edit selection"
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {},
            "additionalProperties": false
        })
    }

    async fn execute(&self, input: Value) -> ToolResult {
        if input != json!({}) {
            return ToolResult::error("timeline.get accepts no arguments");
        }
        let default_scope =
            match resolve_revision_scope(&self.turn.film, self.turn.selected_scene, None) {
                Ok(RevisionScope::Project) => "project",
                Ok(RevisionScope::Scene(_)) => "selected_scene",
                Err(error) => return ToolResult::error(error.to_string()),
            };
        self.turn.state.inspected.store(true, Ordering::SeqCst);
        let film = &self.turn.film;
        let scenes: Vec<_> = film
            .timeline
            .scenes
            .iter()
            .map(|scene| {
                json!({
                    "id": scene.id,
                    "title": scene.title,
                    "asset_id": scene.asset_id,
                    "source": scene.source,
                    "duration_ms": scene.source.duration_ms(),
                    "locked": scene.locked
                })
            })
            .collect();
        ToolResult::success(
            json!({
                "selected_scene_id": self.turn.selected_scene,
                "default_edit_scope": default_scope,
                "explicit_whole_video_requested": explicitly_whole_video(&self.turn.request),
                "duration_ms": film.timeline.duration_ms(),
                "scenes": scenes
            })
            .to_string(),
        )
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn nature(&self) -> ToolNature {
        ToolNature::Read
    }
}

#[derive(Clone)]
pub struct TimelineEditTool {
    turn: RevisionTurn,
}

impl TimelineEditTool {
    pub(crate) fn new(turn: RevisionTurn) -> Self {
        Self { turn }
    }

    fn prepare(&self, value: Value) -> Result<Value> {
        ensure!(
            self.turn.state.inspected.load(Ordering::SeqCst),
            "inspect timeline.get before preparing a revision"
        );
        let input: EditInput =
            serde_json::from_value(value).context("invalid timeline.edit input")?;

        ensure!(
            !has_negative_instruction(&self.turn.request),
            "negative or ambiguous revision request; no edit staged"
        );
        ensure!(
            operation_supported_by_request(&self.turn.request, input.operation),
            "requested edit is not in the supported deterministic revision subset"
        );

        let explicit_whole = explicitly_whole_video(&self.turn.request);
        if input.scope == Some(RequestedScope::WholeVideo) {
            ensure!(
                explicit_whole || self.turn.selected_scene.is_none(),
                "cannot override the selected scene without explicit whole-video wording"
            );
        }
        // The original user's explicit words win even when a model omits the
        // scope field. The model can never manufacture an override.
        let explicit_scope = if explicit_whole {
            Some(RevisionScope::Project)
        } else {
            None
        };
        let resolved =
            resolve_revision_scope(&self.turn.film, self.turn.selected_scene, explicit_scope)?;
        let mut next = self.turn.film.clone();

        match (input.operation, resolved) {
            (EditOperation::Shorten, RevisionScope::Scene(scene_id)) => {
                let scene = next
                    .timeline
                    .scenes
                    .iter()
                    .find(|scene| scene.id == scene_id)
                    .context("selected scene is stale")?;
                let new_source = shorten_source(&scene.source)?;
                next = next.revised(
                    &self.turn.project,
                    RevisionAction::Trim {
                        scene_id,
                        source: new_source,
                    },
                    RevisionOrigin::Automatic,
                )?;
            }
            (EditOperation::Shorten, RevisionScope::Project) => {
                let unlocked: Vec<_> = next
                    .timeline
                    .scenes
                    .iter()
                    .filter(|scene| !scene.locked)
                    .map(|scene| (scene.id, scene.source.clone()))
                    .collect();
                ensure!(
                    !unlocked.is_empty(),
                    "all scenes are locked; no edit staged"
                );
                for (scene_id, source) in unlocked {
                    next = next.revised(
                        &self.turn.project,
                        RevisionAction::Trim {
                            scene_id,
                            source: shorten_source(&source)?,
                        },
                        RevisionOrigin::Automatic,
                    )?;
                }
            }
            (EditOperation::Delete, RevisionScope::Scene(scene_id)) => {
                next = next.revised(
                    &self.turn.project,
                    RevisionAction::Delete { scene_id },
                    RevisionOrigin::Automatic,
                )?;
            }
            (EditOperation::Delete, RevisionScope::Project) => {
                bail!("deleting the whole video is not a supported revision");
            }
        }

        ensure!(next != self.turn.film, "revision would not change the film");
        let before_duration_ms = self.turn.film.timeline.duration_ms();
        let after_duration_ms = next.timeline.duration_ms();
        let scene_count = next.timeline.scenes.len();

        let mut prepared = self
            .turn
            .state
            .prepared
            .lock()
            .map_err(|_| anyhow::anyhow!("revision staging mutex is poisoned"))?;
        ensure!(
            prepared.is_none(),
            "only one edit is permitted per Talos turn"
        );
        *prepared = Some(next);

        Ok(json!({
            "status": "prepared_not_committed",
            "scope": match resolved {
                RevisionScope::Project => "project",
                RevisionScope::Scene(_) => "selected_scene"
            },
            "before_duration_ms": before_duration_ms,
            "after_duration_ms": after_duration_ms,
            "scene_count": scene_count
        }))
    }
}

#[async_trait]
impl AgentTool for TimelineEditTool {
    fn name(&self) -> &str {
        TIMELINE_EDIT_TOOL
    }

    fn description(&self) -> &str {
        "Prepare one safe scene revision: shorten selected/whole film or delete the selected scene"
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["shorten", "delete"],
                    "description": "Only the two supported deterministic edit intents"
                },
                "scope": {
                    "type": "string",
                    "enum": ["default", "whole_video"],
                    "description": "Default uses selection. whole_video must be explicit in the original user request."
                }
            },
            "required": ["operation"],
            "additionalProperties": false
        })
    }

    async fn execute(&self, input: Value) -> ToolResult {
        match self.prepare(input) {
            Ok(value) => ToolResult::success(value.to_string()),
            Err(error) => {
                self.turn.state.rejected.store(true, Ordering::SeqCst);
                ToolResult::error(error.to_string())
            }
        }
    }

    fn nature(&self) -> ToolNature {
        ToolNature::Write
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
enum EditOperation {
    Shorten,
    Delete,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
enum RequestedScope {
    Default,
    WholeVideo,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EditInput {
    operation: EditOperation,
    #[serde(default)]
    scope: Option<RequestedScope>,
}

fn shorten_source(source: &SceneSource) -> Result<SceneSource> {
    let previous_duration = source.duration_ms();
    ensure!(
        previous_duration > 1,
        "scene is already at minimum positive duration"
    );
    // Deliberately conservative, fixed MVP assumption (20% shortening);
    // the model does not get to invent arbitrary source ranges.
    let decrease_ms = (previous_duration / 5).max(1);
    let new_duration = previous_duration - decrease_ms;
    Ok(match source {
        SceneSource::Image { .. } => SceneSource::Image {
            duration_ms: new_duration,
        },
        SceneSource::Video { start_ms, .. } => SceneSource::Video {
            start_ms: *start_ms,
            end_ms: start_ms
                .checked_add(new_duration)
                .context("shortened video range overflow")?,
        },
    })
}

fn explicitly_whole_video(request: &str) -> bool {
    let request = request.to_lowercase();
    [
        "整个视频",
        "整部视频",
        "整部影片",
        "整段视频",
        "整部片",
        "全片",
        "所有片段",
        "全部片段",
        "whole video",
        "entire video",
        "whole film",
        "entire film",
        "all scenes",
        "all clips",
    ]
    .iter()
    .any(|phrase| request.contains(phrase))
}

fn has_negative_instruction(request: &str) -> bool {
    let request = request.to_lowercase();
    [
        "不要", "别", "不许", "禁止", "do not", "don't", "not ", "never",
    ]
    .iter()
    .any(|phrase| request.contains(phrase))
}

fn operation_supported_by_request(request: &str, operation: EditOperation) -> bool {
    let request = request.to_lowercase();
    let phrases: &[&str] = match operation {
        EditOperation::Shorten => &["缩短", "短一点", "短一些", "剪短", "更短", "short", "trim"],
        EditOperation::Delete => &["删除", "删掉", "删了", "移除", "去掉", "delete", "remove"],
    };
    phrases.iter().any(|phrase| request.contains(phrase))
}
