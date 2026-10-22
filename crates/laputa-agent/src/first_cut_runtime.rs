use std::sync::Arc;

use anyhow::{Context, Result, bail, ensure};
use laputa_media::ProjectStore;
use laputa_timeline::{FilmState, FilmStore};
use talos_runtime::{
    AgentEvent, LanguageModel, PermissionDecision, PermissionRule, RuntimeBuilder, RuntimeHandle,
    RuntimeTurnCompletionStatus, SessionEvent, TurnEventPayload,
};

use crate::first_cut::{
    CommitFirstCutTool, FILM_COMMIT_FIRST_CUT_TOOL, PROJECT_INSPECT_TOOL, ProjectInspectTool,
};

const FIRST_CUT_SYSTEM_PROMPT: &str = r#"You are Laputa's First Cut Director.

Your job is to turn the user's creative request into one coherent initial film state. Work only through the Laputa tools provided to you.

Required behavior:
- Inspect the project with `project.inspect` before committing a first cut.
- Treat imported Asset IDs and canonical Ready analyses returned by that tool as authoritative. Never invent Asset IDs or source durations.
- Build a concise creative intent, an ordered story with semantic beats, and an ordered scene list.
- Use `film.commit_first_cut` to commit the complete draft atomically. Laputa generates stable beat and scene IDs and validates all media references and source ranges.
- If the available project information is insufficient for a valid first cut, do not pretend that a commit succeeded.
- After a successful commit, respond briefly. Do not expose model/provider/workflow internals or chain-of-thought.
"#;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FirstCutProgress {
    Starting,
    UnderstandingMoments,
    FindingStory,
    BuildingScenes,
    Completed,
    Failed,
}

#[derive(Clone)]
pub struct FirstCutRunner {
    project_store: ProjectStore,
    provider: Arc<dyn LanguageModel>,
}

impl FirstCutRunner {
    #[must_use]
    pub fn new(project_store: ProjectStore, provider: Arc<dyn LanguageModel>) -> Self {
        Self {
            project_store,
            provider,
        }
    }

    pub async fn run(&self, request: impl Into<String>) -> Result<FilmState> {
        self.run_with_progress(request, |_| {}).await
    }

    pub async fn run_with_progress<F>(
        &self,
        request: impl Into<String>,
        mut on_progress: F,
    ) -> Result<FilmState>
    where
        F: FnMut(FirstCutProgress),
    {
        let request = request.into();
        let mut last_progress = None;
        emit_progress(
            &mut on_progress,
            &mut last_progress,
            FirstCutProgress::Starting,
        );

        let result = self
            .run_inner(&request, &mut on_progress, &mut last_progress)
            .await;
        emit_progress(
            &mut on_progress,
            &mut last_progress,
            if result.is_ok() {
                FirstCutProgress::Completed
            } else {
                FirstCutProgress::Failed
            },
        );
        result
    }

    async fn run_inner<F>(
        &self,
        request: &str,
        on_progress: &mut F,
        last_progress: &mut Option<FirstCutProgress>,
    ) -> Result<FilmState>
    where
        F: FnMut(FirstCutProgress),
    {
        ensure!(!request.trim().is_empty(), "first-cut request must not be empty");

        let project_before = self.project_store.open()?;
        let film_store = FilmStore::at(self.project_store.root());
        let state_before = film_store.load(&project_before)?;

        let mut runtime = RuntimeBuilder::new()
            .provider(self.provider.clone())
            .workspace_root(self.project_store.root())
            .custom_prompt(FIRST_CUT_SYSTEM_PROMPT)
            .tool(Arc::new(ProjectInspectTool::new(self.project_store.clone())))
            .tool(Arc::new(CommitFirstCutTool::new(
                self.project_store.clone(),
                request,
            )))
            .permission_rule(PermissionRule::new(
                FILM_COMMIT_FIRST_CUT_TOOL,
                None,
                PermissionDecision::Allow,
            ))
            .build()
            .context("failed to build Talos first-cut runtime")?;

        if let Err(error) = runtime.submit(request.to_owned()).await {
            let _ = runtime.shutdown().await;
            return Err(error).context("failed to submit first-cut request to Talos");
        }

        let completion = drive_until_completed(&mut runtime, on_progress, last_progress).await;
        let shutdown = runtime.shutdown().await;
        let completion = match completion {
            Ok(status) => {
                shutdown.context("failed to shut down Talos first-cut runtime")?;
                status
            }
            Err(error) => {
                let _ = shutdown;
                return Err(error);
            }
        };

        match completion {
            RuntimeTurnCompletionStatus::Success { .. } => {}
            RuntimeTurnCompletionStatus::Cancelled => {
                bail!("Talos first-cut turn was cancelled");
            }
            RuntimeTurnCompletionStatus::Error { message } => {
                bail!("Talos first-cut turn failed: {message}");
            }
        }

        let project_after = self.project_store.open()?;
        let state_after = film_store
            .load(&project_after)?
            .context("Talos turn completed without a committed FilmState")?;
        ensure!(
            state_before.as_ref() != Some(&state_after),
            "Talos turn completed without committing a new FilmState"
        );
        ensure!(
            state_after.intent.request == request,
            "committed FilmState does not belong to the submitted request"
        );
        Ok(state_after)
    }
}

async fn drive_until_completed<F>(
    runtime: &mut RuntimeHandle,
    on_progress: &mut F,
    last_progress: &mut Option<FirstCutProgress>,
) -> Result<RuntimeTurnCompletionStatus>
where
    F: FnMut(FirstCutProgress),
{
    let mut inspect_call_id = None;
    let mut last_session_error = None;

    while let Some(event) = runtime.next_event().await {
        match event {
            SessionEvent::TurnEvent { payload, .. } => match payload {
                TurnEventPayload::Started => {}
                TurnEventPayload::Progress { event } => match event {
                    AgentEvent::ToolCall { call, .. } if call.name == PROJECT_INSPECT_TOOL => {
                        inspect_call_id = Some(call.id);
                        emit_progress(
                            on_progress,
                            last_progress,
                            FirstCutProgress::UnderstandingMoments,
                        );
                    }
                    AgentEvent::ToolResult { result, .. }
                        if !result.is_error
                            && inspect_call_id.as_deref() == Some(&result.tool_use_id) =>
                    {
                        emit_progress(
                            on_progress,
                            last_progress,
                            FirstCutProgress::FindingStory,
                        );
                    }
                    AgentEvent::ToolCall { call, .. }
                        if call.name == FILM_COMMIT_FIRST_CUT_TOOL =>
                    {
                        emit_progress(
                            on_progress,
                            last_progress,
                            FirstCutProgress::BuildingScenes,
                        );
                    }
                    _ => {}
                },
                TurnEventPayload::Completed { status } => return Ok(status),
                _ => {}
            },
            SessionEvent::Error { message } => last_session_error = Some(message),
            _ => {}
        }
    }

    if let Some(message) = last_session_error {
        bail!("Talos runtime ended before turn completion: {message}");
    }
    bail!("Talos runtime ended before turn completion");
}

fn emit_progress<F>(
    on_progress: &mut F,
    last_progress: &mut Option<FirstCutProgress>,
    progress: FirstCutProgress,
) where
    F: FnMut(FirstCutProgress),
{
    if *last_progress == Some(progress) {
        return;
    }
    on_progress(progress);
    *last_progress = Some(progress);
}

#[cfg(test)]
mod tests {
    use std::{
        collections::VecDeque,
        fs,
        path::Path,
        sync::{Arc, Mutex},
    };

    use anyhow::Result;
    use async_trait::async_trait;
    use laputa_domain::{AssetAnalysis, AssetTechnicalMetadata, MediaKind};
    use laputa_media::{MediaProbe, ProjectStore};
    use laputa_timeline::{
        FilmDraft, FilmState, FilmStore, IntentDraft, SceneDraft, SceneSource, StoryBeatDraft,
        StoryDraft,
    };
    use talos_runtime::{
        AgentEvent, CapabilityProbe, LanguageModel, Message, ProtocolCapabilities, ProviderError,
        ProviderResult, Receiver, StopReason, ToolCall, ToolProvenance, Usage,
    };

    use super::{FirstCutProgress, FirstCutRunner};
    use crate::first_cut::{FILM_COMMIT_FIRST_CUT_TOOL, PROJECT_INSPECT_TOOL};

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

    #[derive(Default)]
    struct ScriptedProvider {
        responses: Mutex<VecDeque<Vec<AgentEvent>>>,
    }

    impl ScriptedProvider {
        fn new() -> Self {
            Self::default()
        }

        fn with_tool_call(self, id: &str, name: &str, input: serde_json::Value) -> Self {
            self.responses
                .lock()
                .expect("scripted provider mutex")
                .push_back(vec![
                    AgentEvent::TurnStart,
                    AgentEvent::ToolCall {
                        call: ToolCall {
                            id: id.to_owned(),
                            name: name.to_owned(),
                            input,
                        },
                        provenance: ToolProvenance::Native,
                        summary_fields: Vec::new(),
                    },
                    AgentEvent::TurnEnd {
                        stop_reason: StopReason::ToolUse,
                        usage: Usage::default(),
                    },
                ]);
            self
        }

        fn with_response(self, text: &str) -> Self {
            self.responses
                .lock()
                .expect("scripted provider mutex")
                .push_back(vec![
                    AgentEvent::TurnStart,
                    AgentEvent::TextDelta {
                        delta: text.to_owned(),
                    },
                    AgentEvent::TurnEnd {
                        stop_reason: StopReason::EndTurn,
                        usage: Usage::default(),
                    },
                ]);
            self
        }
    }

    #[async_trait]
    impl LanguageModel for ScriptedProvider {
        fn protocol_capabilities(&self) -> CapabilityProbe {
            CapabilityProbe::Known(ProtocolCapabilities {
                native_tools: true,
                compatibility: false,
            })
        }

        async fn stream(&self, _messages: &[Message]) -> ProviderResult<Receiver<AgentEvent>> {
            let events = self
                .responses
                .lock()
                .map_err(|_| ProviderError::InvalidResponse("fixture mutex poisoned".into()))?
                .pop_front()
                .ok_or_else(|| {
                    ProviderError::InvalidResponse("fixture response exhausted".into())
                })?;
            let (tx, rx) = tokio::sync::mpsc::channel(events.len().max(1));
            for event in events {
                tx.try_send(event).map_err(|_| {
                    ProviderError::InvalidResponse("fixture channel unavailable".into())
                })?;
            }
            Ok(rx)
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

    fn successful_provider(store: &ProjectStore) -> Result<ScriptedProvider> {
        Ok(ScriptedProvider::new()
            .with_tool_call("inspect-1", PROJECT_INSPECT_TOOL, serde_json::json!({}))
            .with_tool_call(
                "commit-1",
                FILM_COMMIT_FIRST_CUT_TOOL,
                serde_json::to_value(draft(store)?)?,
            )
            .with_response("First cut ready"))
    }

    #[tokio::test]
    async fn deterministic_talos_loop_commits_first_cut_and_projects_progress() -> Result<()> {
        let (_temp, store) = prepared_project()?;
        let runner = FirstCutRunner::new(store.clone(), Arc::new(successful_provider(&store)?));
        let mut progress = Vec::new();

        let state = runner
            .run_with_progress("Make a warm one-minute Kyoto vlog", |event| {
                progress.push(event);
            })
            .await?;

        assert_eq!(state.intent.request, "Make a warm one-minute Kyoto vlog");
        assert_eq!(state.timeline.scenes.len(), 2);
        assert_eq!(
            progress,
            vec![
                FirstCutProgress::Starting,
                FirstCutProgress::UnderstandingMoments,
                FirstCutProgress::FindingStory,
                FirstCutProgress::BuildingScenes,
                FirstCutProgress::Completed,
            ]
        );
        assert_eq!(
            FilmStore::at(store.root()).load(&store.open()?)?,
            Some(state)
        );
        Ok(())
    }

    #[tokio::test]
    async fn rejected_commit_finishes_as_failure_without_persisting_state() -> Result<()> {
        let (_temp, store) = prepared_project()?;
        let mut invalid = draft(&store)?;
        invalid.scenes[0].asset_id = laputa_domain::AssetId::new();
        let provider = ScriptedProvider::new()
            .with_tool_call("inspect-1", PROJECT_INSPECT_TOOL, serde_json::json!({}))
            .with_tool_call(
                "commit-1",
                FILM_COMMIT_FIRST_CUT_TOOL,
                serde_json::to_value(invalid)?,
            )
            .with_response("Done");
        let runner = FirstCutRunner::new(store.clone(), Arc::new(provider));
        let mut progress = Vec::new();

        let result = runner
            .run_with_progress("Make a vlog", |event| progress.push(event))
            .await;

        assert!(result.is_err());
        assert_eq!(progress.last(), Some(&FirstCutProgress::Failed));
        assert!(!FilmStore::at(store.root()).path().exists());
        Ok(())
    }

    #[tokio::test]
    async fn successful_turn_cannot_reuse_an_old_film_state() -> Result<()> {
        let (_temp, store) = prepared_project()?;
        let project = store.open()?;
        let existing = FilmState::from_draft("Old request", draft(&store)?, &project)?;
        let film_store = FilmStore::at(store.root());
        film_store.save(&project, &existing)?;
        let runner = FirstCutRunner::new(
            store.clone(),
            Arc::new(ScriptedProvider::new().with_response("Done without a commit")),
        );

        let result = runner.run("New request").await;

        assert!(result.is_err());
        assert_eq!(film_store.load(&store.open()?)?, Some(existing));
        Ok(())
    }
}
