//! A bounded Talos turn for editing an existing film.
//! Models can propose a single supported operation; only a completed turn with
//! no rejected Tool calls may commit the staged FilmState.

use std::sync::{Arc, atomic::Ordering};

use anyhow::{Context, Result, bail, ensure};
use laputa_media::ProjectStore;
use laputa_timeline::{FilmState, FilmStore, SceneId, revision::resolve_revision_scope};
use talos_runtime::{
    LanguageModel, PermissionDecision, PermissionRule, RuntimeBuilder, RuntimeTurnCompletionStatus,
    SessionEvent, TurnEventPayload,
};

use crate::revision_tools::{RevisionTurn, TIMELINE_EDIT_TOOL, TimelineEditTool, TimelineGetTool};

const REVISION_PROMPT: &str = r#"You are Laputa's constrained Film Revision Director.

The user's real language and current Selection context determine the target.
- Always inspect the existing film with timeline.get before suggesting an edit.
- Use timeline.edit only for a supported edit: shorten a scene/video or delete one selected scene.
- A selected scene is the default edit target. Without selection, the whole video is the target.
- Explicit whole-video language overrides selection; you cannot claim it was explicit if it was not in the user's request.
- Do not fabricate Scene IDs, time ranges, provider controls, or media-generation results.
- A prepared edit is NOT committed until the Laputa runner confirms a successful turn.
- If the user requests an unsupported effect such as warmth/style changes, explain it is not supported here; do not pretend it was applied.
- If the requested operation or edit scope is uncertain, do not call timeline.edit.
- Report a result only after the tools actually succeed. Do not expose model/provider/workflow choices.
"#;

#[derive(Clone)]
pub struct RevisionRunner {
    project_store: ProjectStore,
    provider: Arc<dyn LanguageModel>,
}

impl RevisionRunner {
    #[must_use]
    pub fn new(project_store: ProjectStore, provider: Arc<dyn LanguageModel>) -> Self {
        Self {
            project_store,
            provider,
        }
    }

    /// Run one natural-language revision with the product's captured Scene selection.
    ///
    /// F2 is an integration entrypoint. GUI command wiring and state re-projection
    /// are owned by F3; callers must pass the actual selected Scene ID.
    pub async fn run(
        &self,
        request: impl Into<String>,
        selected_scene: Option<SceneId>,
    ) -> Result<FilmState> {
        let request = request.into();
        ensure!(
            !request.trim().is_empty(),
            "revision request must not be empty"
        );

        let project_before = self.project_store.open()?;
        let film_store = FilmStore::at(self.project_store.root());
        let film_before = film_store
            .load(&project_before)?
            .context("revision requires a committed first cut")?;
        // Selection validity is checked before any model invocation.
        resolve_revision_scope(&film_before, selected_scene, None)?;

        let turn = RevisionTurn::new(
            project_before.clone(),
            film_before.clone(),
            selected_scene,
            request.clone(),
        );

        let mut runtime = RuntimeBuilder::new()
            .provider(self.provider.clone())
            .workspace_root(self.project_store.root())
            .custom_prompt(REVISION_PROMPT)
            .tool(Arc::new(TimelineGetTool::new(turn.clone())))
            .tool(Arc::new(TimelineEditTool::new(turn.clone())))
            .permission_rule(PermissionRule::new(
                TIMELINE_EDIT_TOOL,
                None,
                PermissionDecision::Allow,
            ))
            .build()
            .context("failed to build Talos revision runtime")?;

        if let Err(error) = runtime.submit(request).await {
            let _ = runtime.shutdown().await;
            return Err(error).context("failed to submit revision request to Talos");
        }

        let mut session_error = None;
        let mut completion = None;
        while let Some(event) = runtime.next_event().await {
            match event {
                SessionEvent::TurnEvent {
                    payload: TurnEventPayload::Completed { status },
                    ..
                } => {
                    completion = Some(status);
                    break;
                }
                SessionEvent::Error { message } => session_error = Some(message),
                _ => {}
            }
        }
        let shutdown_result = runtime.shutdown().await;
        match completion {
            Some(RuntimeTurnCompletionStatus::Success { .. }) => {
                shutdown_result.context("failed to shut down revision runtime")?;
            }
            Some(RuntimeTurnCompletionStatus::Cancelled) => {
                bail!("Talos revision turn was cancelled without committing");
            }
            Some(RuntimeTurnCompletionStatus::Error { message }) => {
                bail!("Talos revision turn failed without committing: {message}");
            }
            None => {
                bail!(
                    "Talos revision turn ended without completing: {}",
                    session_error.unwrap_or_else(|| "missing completion event".to_owned())
                );
            }
        }

        ensure!(
            !turn.state.rejected.load(Ordering::SeqCst),
            "Talos revision contained a rejected edit; no changes committed"
        );
        let next = turn
            .state
            .prepared
            .lock()
            .map_err(|_| anyhow::anyhow!("revision staging mutex is poisoned"))?
            .clone()
            .context("Talos revision completed without a valid prepared edit")?;

        // No overwrites when a different writer changed the project while the
        // model was thinking. This is a single-writer baseline guard, not an
        // atomic compare-and-swap guarantee for simultaneous writers.
        let project_after = self.project_store.open()?;
        ensure!(
            project_after == project_before,
            "project changed during revision; refusing stale edit"
        );
        let film_after = film_store.load(&project_after)?;
        ensure!(
            film_after.as_ref() == Some(&film_before),
            "film changed during revision; refusing stale edit"
        );
        film_store.save(&project_after, &next)?;
        Ok(next)
    }
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
    use laputa_domain::{AssetTechnicalMetadata, MediaKind};
    use laputa_media::{MediaProbe, ProjectStore};
    use laputa_timeline::{
        FilmDraft, FilmState, FilmStore, IntentDraft, SceneDraft, SceneSource, StoryBeatDraft,
        StoryDraft,
        revision::{RevisionAction, RevisionOrigin},
    };
    use serde_json::{Value, json};
    use talos_runtime::{
        AgentEvent, AgentTool, CapabilityProbe, LanguageModel, Message, ProtocolCapabilities,
        ProviderError, ProviderResult, Receiver, StopReason, ToolCall, ToolProvenance, Usage,
    };

    use super::RevisionRunner;
    use crate::revision_tools::{
        RevisionTurn, TIMELINE_EDIT_TOOL, TIMELINE_GET_TOOL, TimelineEditTool, TimelineGetTool,
    };

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
                    duration_ms: Some(9_000),
                    video_codec: Some("fixture".to_owned()),
                    has_audio: false,
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

        fn with_tool_call(self, id: &str, name: &str, input: Value) -> Self {
            self.responses.lock().expect("script mutex").push_back(vec![
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
            self.responses.lock().expect("script mutex").push_back(vec![
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
                .map_err(|_| ProviderError::InvalidResponse("script mutex poisoned".into()))?
                .pop_front()
                .ok_or_else(|| ProviderError::InvalidResponse("script exhausted".into()))?;
            let (tx, rx) = tokio::sync::mpsc::channel(events.len().max(1));
            for event in events {
                tx.try_send(event).map_err(|_| {
                    ProviderError::InvalidResponse("scripted channel unavailable".into())
                })?;
            }
            Ok(rx)
        }
    }

    fn fixture() -> Result<(tempfile::TempDir, ProjectStore, FilmState)> {
        let temp = tempfile::tempdir()?;
        let photo = temp.path().join("photo.jpg");
        let video = temp.path().join("video.mp4");
        fs::write(&photo, b"original-photo-sentinel")?;
        fs::write(&video, b"original-video-sentinel")?;
        let store = ProjectStore::at(temp.path().join("project"));
        store.create("Trip")?;
        let imported = store.import_media(&[photo, video], &FixtureProbe)?;
        let project = store.open()?;
        let state = FilmState::from_draft(
            "Make a two-part film",
            FilmDraft {
                intent: IntentDraft {
                    summary: "Travel memories".to_owned(),
                    target_duration_ms: Some(12_000),
                    tone_tags: vec!["natural".to_owned()],
                },
                story: StoryDraft {
                    summary: "Arrival, then walk".to_owned(),
                    beats: vec![
                        StoryBeatDraft {
                            key: "arrive".to_owned(),
                            title: "Arrival".to_owned(),
                            purpose: "Opening".to_owned(),
                        },
                        StoryBeatDraft {
                            key: "walk".to_owned(),
                            title: "Walk".to_owned(),
                            purpose: "Explore".to_owned(),
                        },
                    ],
                },
                scenes: vec![
                    SceneDraft {
                        beat_key: "arrive".to_owned(),
                        asset_id: imported.imported[0].id,
                        title: "Photo".to_owned(),
                        source: SceneSource::Image { duration_ms: 4_000 },
                    },
                    SceneDraft {
                        beat_key: "walk".to_owned(),
                        asset_id: imported.imported[1].id,
                        title: "Video".to_owned(),
                        source: SceneSource::Video {
                            start_ms: 1_000,
                            end_ms: 7_000,
                        },
                    },
                ],
            },
            &project,
        )?;
        FilmStore::at(store.root()).save(&project, &state)?;
        Ok((temp, store, state))
    }

    fn runner(store: &ProjectStore, provider: ScriptedProvider) -> RevisionRunner {
        RevisionRunner::new(store.clone(), Arc::new(provider))
    }

    fn scripted_edit(operation: &str, scope: Option<&str>) -> ScriptedProvider {
        let mut input = json!({"operation": operation});
        if let Some(scope) = scope {
            input["scope"] = json!(scope);
        }
        ScriptedProvider::new()
            .with_tool_call("get-1", TIMELINE_GET_TOOL, json!({}))
            .with_tool_call("edit-1", TIMELINE_EDIT_TOOL, input)
            .with_response("Prepared")
    }

    #[tokio::test]
    async fn selected_scene_shortens_after_real_talos_turn_and_preserves_other_scene() -> Result<()>
    {
        let (temp, store, before) = fixture()?;
        let selected = before.timeline.scenes[1].id;
        let edited = runner(&store, scripted_edit("shorten", None))
            .run("把这里短一点", Some(selected))
            .await?;
        assert_eq!(edited.timeline.scenes[0], before.timeline.scenes[0]);
        assert_eq!(edited.timeline.scenes[1].source.duration_ms(), 4_800);
        assert_eq!(edited.timeline.duration_ms(), 8_800);
        assert_eq!(
            FilmStore::at(store.root()).load(&store.open()?)?,
            Some(edited)
        );
        assert_eq!(
            fs::read(temp.path().join("photo.jpg"))?,
            b"original-photo-sentinel"
        );
        assert_eq!(
            fs::read(temp.path().join("video.mp4"))?,
            b"original-video-sentinel"
        );
        Ok(())
    }

    #[tokio::test]
    async fn no_selection_defaults_to_whole_video() -> Result<()> {
        let (_temp, store, before) = fixture()?;
        let edited = runner(&store, scripted_edit("shorten", None))
            .run("把视频缩短一点", None)
            .await?;
        assert_eq!(edited.timeline.scenes[0].source.duration_ms(), 3_200);
        assert_eq!(edited.timeline.scenes[1].source.duration_ms(), 4_800);
        assert_ne!(before.timeline, edited.timeline);
        assert_eq!(
            FilmStore::at(store.root()).load(&store.open()?)?,
            Some(edited)
        );
        Ok(())
    }

    #[tokio::test]
    async fn locked_selected_scene_rejects_talos_edit_without_committing() -> Result<()> {
        let (_temp, store, before) = fixture()?;
        let project = store.open()?;
        let selected = before.timeline.scenes[1].id;
        let baseline = FilmStore::at(store.root()).apply_revision(
            &project,
            RevisionAction::SetLocked {
                scene_id: selected,
                locked: true,
            },
            RevisionOrigin::User,
        )?;

        let result = runner(&store, scripted_edit("shorten", None))
            .run("把这里短一点", Some(selected))
            .await;
        assert!(result.is_err());
        assert_eq!(
            FilmStore::at(store.root()).load(&store.open()?)?,
            Some(baseline)
        );
        Ok(())
    }

    #[tokio::test]
    async fn explicit_whole_video_wording_overrides_selected_scene_without_model_scope()
    -> Result<()> {
        let (_temp, store, before) = fixture()?;
        let selected = before.timeline.scenes[1].id;
        let edited = runner(&store, scripted_edit("shorten", None))
            .run("把整个视频缩短一点", Some(selected))
            .await?;
        assert_eq!(edited.timeline.scenes[0].source.duration_ms(), 3_200);
        assert_eq!(edited.timeline.scenes[1].source.duration_ms(), 4_800);
        assert_eq!(edited.timeline.duration_ms(), 8_000);
        Ok(())
    }

    #[tokio::test]
    async fn global_shortening_keeps_locked_scene_byte_for_byte_unchanged() -> Result<()> {
        let (_temp, store, original) = fixture()?;
        let project = store.open()?;
        let first_id = original.timeline.scenes[0].id;
        let baseline = FilmStore::at(store.root()).apply_revision(
            &project,
            RevisionAction::SetLocked {
                scene_id: first_id,
                locked: true,
            },
            RevisionOrigin::User,
        )?;
        let edited = runner(&store, scripted_edit("shorten", Some("whole_video")))
            .run("让整个视频短一点", Some(baseline.timeline.scenes[1].id))
            .await?;
        assert_eq!(edited.timeline.scenes[0], baseline.timeline.scenes[0]);
        assert_eq!(edited.timeline.scenes[1].source.duration_ms(), 4_800);
        Ok(())
    }

    #[tokio::test]
    async fn model_cannot_invent_global_override_or_apply_unsupported_warmth() -> Result<()> {
        let (_temp, store, before) = fixture()?;
        let selected = before.timeline.scenes[1].id;

        let wrong_scope = runner(&store, scripted_edit("shorten", Some("whole_video")))
            .run("把这里短一点", Some(selected))
            .await;
        assert!(wrong_scope.is_err());
        assert_eq!(
            FilmStore::at(store.root()).load(&store.open()?)?,
            Some(before.clone())
        );

        let unsupported = runner(&store, scripted_edit("shorten", Some("whole_video")))
            .run("让整个视频更暖一点", Some(selected))
            .await;
        assert!(unsupported.is_err());
        assert_eq!(
            FilmStore::at(store.root()).load(&store.open()?)?,
            Some(before)
        );
        Ok(())
    }

    #[tokio::test]
    async fn model_failure_after_prepared_edit_does_not_commit() -> Result<()> {
        let (_temp, store, before) = fixture()?;
        let selected = before.timeline.scenes[1].id;
        let incomplete = ScriptedProvider::new()
            .with_tool_call("get-1", TIMELINE_GET_TOOL, json!({}))
            .with_tool_call("edit-1", TIMELINE_EDIT_TOOL, json!({"operation":"shorten"}));
        let result = runner(&store, incomplete)
            .run("把这里短一点", Some(selected))
            .await;
        assert!(result.is_err());
        assert_eq!(
            FilmStore::at(store.root()).load(&store.open()?)?,
            Some(before)
        );
        Ok(())
    }

    #[tokio::test]
    async fn rejected_second_tool_call_aborts_prepared_first_edit() -> Result<()> {
        let (_temp, store, before) = fixture()?;
        let selected = before.timeline.scenes[1].id;
        let provider = ScriptedProvider::new()
            .with_tool_call("get-1", TIMELINE_GET_TOOL, json!({}))
            .with_tool_call("edit-1", TIMELINE_EDIT_TOOL, json!({"operation":"shorten"}))
            .with_tool_call("edit-2", TIMELINE_EDIT_TOOL, json!({"operation":"delete"}))
            .with_response("Done");
        let result = runner(&store, provider)
            .run("把这里短一点", Some(selected))
            .await;
        assert!(result.is_err());
        assert_eq!(
            FilmStore::at(store.root()).load(&store.open()?)?,
            Some(before)
        );
        Ok(())
    }

    #[tokio::test]
    async fn editing_without_first_inspecting_is_rejected() -> Result<()> {
        let (_temp, store, before) = fixture()?;
        let selected = before.timeline.scenes[1].id;
        let provider = ScriptedProvider::new()
            .with_tool_call("edit-1", TIMELINE_EDIT_TOOL, json!({"operation":"shorten"}))
            .with_response("Done");
        assert!(
            runner(&store, provider)
                .run("把这里短一点", Some(selected))
                .await
                .is_err()
        );
        assert_eq!(
            FilmStore::at(store.root()).load(&store.open()?)?,
            Some(before)
        );
        Ok(())
    }

    #[tokio::test]
    async fn no_tool_call_or_stale_selection_does_not_mutate_film() -> Result<()> {
        let (_temp, store, before) = fixture()?;
        let no_tool = ScriptedProvider::new().with_response("I did it");
        assert!(
            runner(&store, no_tool)
                .run("把这里短一点", None)
                .await
                .is_err()
        );

        let stale_selection = runner(&store, scripted_edit("shorten", None))
            .run("把这里短一点", Some(laputa_timeline::SceneId::new()))
            .await;
        assert!(stale_selection.is_err());
        assert_eq!(
            FilmStore::at(store.root()).load(&store.open()?)?,
            Some(before)
        );
        Ok(())
    }

    #[tokio::test]
    async fn delete_selected_scene_is_non_destructive_and_keeps_one_scene() -> Result<()> {
        let (_temp, store, before) = fixture()?;
        let selected = before.timeline.scenes[1].id;
        let deleted = runner(&store, scripted_edit("delete", None))
            .run("删除这个片段", Some(selected))
            .await?;
        assert_eq!(deleted.timeline.scenes.len(), 1);
        assert_eq!(deleted.story.beats.len(), 1);
        assert_eq!(deleted.timeline.scenes[0], before.timeline.scenes[0]);
        assert_eq!(store.open()?.assets.len(), 2);
        Ok(())
    }

    #[tokio::test]
    async fn public_tool_schemas_hide_model_controls_and_inspection_hides_source_paths()
    -> Result<()> {
        let (_temp, store, film) = fixture()?;
        let turn = RevisionTurn::new(store.open()?, film, None, "把整个视频短一点".to_owned());
        let get = TimelineGetTool::new(turn.clone());
        let edit = TimelineEditTool::new(turn);
        for schema in [get.parameters(), edit.parameters()] {
            let text = schema.to_string().to_lowercase();
            for forbidden in ["provider", "vendor", "model", "workflow", "sampler"] {
                assert!(!text.contains(forbidden));
            }
        }
        let snapshot = get.execute(json!({})).await;
        assert!(!snapshot.is_error);
        let snapshot: Value = serde_json::from_str(&snapshot.content)?;
        assert_eq!(snapshot["default_edit_scope"], "project");
        assert!(!snapshot.to_string().contains("original_path"));
        assert_eq!(snapshot["scenes"].as_array().map(Vec::len), Some(2));
        Ok(())
    }
}
