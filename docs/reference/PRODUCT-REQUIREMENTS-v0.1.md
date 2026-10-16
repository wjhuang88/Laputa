# Laputa V0.1 Product Requirements

Status: approved initial baseline  
Date: 2026-10-15  
Executable delivery owner: [LAPUTA-001](../backlog/active/LAPUTA-001-agentic-vlog-mvp.md)

## 1. Outcome

V0.1 must prove that an ordinary user can transform a small mixed set of personal photos and videos into a coherent short video through one simple loop:

1. add media;
2. describe the desired video in natural language;
3. receive an automatically assembled first cut;
4. revise by selecting content and describing the desired change, or by simple direct manipulation;
5. export a final video.

The validation target is product behavior and interaction simplicity, not local-model deployment or professional-editor completeness.

## 2. Product constraints

### Hard constraints

- Imported originals are immutable and recoverable throughout normal editing and generation.
- Every external generated artifact is asynchronous state: a submitted job is not treated as a usable asset until verified complete.
- User-visible edit scope must be deterministic: selection by default, project/video when nothing is selected, explicit natural-language scope overrides both.
- An MVP result must be reachable without choosing AI vendor/model, generation workflow, transition algorithm, or professional timeline concepts.
- The application must preserve enough project state to reopen a project and continue from its latest committed project/timeline state.

### Soft constraints

- Rust is the primary application language.
- GPUI is the intended desktop UI stack.
- Talos is the embedded Agent Loop runtime.
- Initial AI capabilities may use external APIs to accelerate product validation; local/open implementations are expected later behind the same functional provider contracts.
- Deterministic media work should prefer FFmpeg/native implementation over generative APIs when generation adds no semantic value.

### Assumptions requiring validation

- A scene-strip abstraction plus selection is sufficient for most V0.1 revisions without a default multi-track timeline.
- A small set of coarse-grained tools gives Talos enough control to produce and repair useful first cuts without exposing low-level operations.
- External multimodal APIs can return structured analysis precise enough for automatic shot selection and critic feedback.
- Users prefer automatic provider/model routing over visible model choices.

Failures of these assumptions must create a Spike or change-control record; they must not silently expand the UI or provider surface.

## 3. Primary user flow

### 3.1 Create project and add media

The user creates/opens a Laputa project and imports local photos/videos by picker or drag-and-drop. Laputa immediately creates project-local metadata/proxy state without altering originals.

### 3.2 State creative intent

The UI presents one primary prompt: “What do you want to make?” The user may describe duration, mood, emphasis, people, pace, story, format, or exclusions in ordinary language.

Laputa converts this into a human-readable `IntentSpec` shown as a compact summary, not a required form. The user can correct it through language or direct edits.

### 3.3 Produce first cut

Talos receives project state plus Laputa tools and autonomously chooses the required analysis, asset selection, story/timeline edits, deterministic processing, and optional generation. The user is not asked to confirm routine, reversible decisions.

The first cut must result in a playable preview and a scene strip representing meaningful sections/clips in order.

### 3.4 Revise

Revision supports both natural language and direct manipulation:

- no selection + “make it faster” changes the whole current video;
- selected scene + “make this faster” affects the selected scene;
- explicit language such as “make the whole video warmer” overrides selection;
- drag reorders scenes/clips;
- trim handles change visible duration;
- delete removes an item from the current timeline without deleting the original;
- lock/retain prevents subsequent automatic revisions from modifying the locked range unless the user explicitly unlocks/overrides it;
- “try another” on a generated result requests another result without exposing vendor/model controls.

### 3.5 Export

The user can export a playable MP4. V0.1 supports at least 16:9 and 9:16 output; format may be inferred/defaulted and can be changed via intent/language without requiring a setup wizard.

## 4. Agent tool surface

The exact schemas may evolve, but V0.1 must provide the following semantic capabilities to Talos:

- `project.inspect`
- `asset.analyze`
- `asset.search`
- `story.get` / `story.update`
- `timeline.get` / `timeline.edit`
- `media.animate_photo`
- `media.generate_broll`
- `media.create_transition`
- `media.stylize`
- `speech.transcribe`
- `speech.generate_narration`
- `render.preview`
- `video.critique`
- `render.final`

Tool implementations may orchestrate multiple functional providers, deterministic media engines, and project-state operations. Low-level provider/model/FFmpeg operations are not part of the normal model-facing tool contract.

## 5. Functional provider contracts

V0.1 code must abstract capabilities by function:

- media/image/video understanding;
- video generation/transformation;
- image generation/editing;
- speech recognition;
- speech synthesis;
- artifact storage/materialization.

Vendor identity is implementation metadata, not a core interface. One vendor may implement several interfaces and a functional interface may have several implementations.

External API adapters are sufficient for V0.1; no local foundation model deployment is required to accept LAPUTA-001.

## 6. Media behavior

- Use deterministic operations for trim, order, scale, crop, encode, basic transition/effect, audio mix and similar reproducible transformations.
- Generative operations are appropriate for photo animation, missing B-roll, semantic style transfer, generative transitions, extension or other operations that require new content.
- `animate_photo` is a product intent, not a synonym for image-to-video; the implementation may use deterministic motion, generative video, or retain the still image.
- Generated variants are separate artifacts and must preserve lineage to their source asset/request/job.

## 7. Critic and repair loop

Laputa must be able to render a proxy, analyze the resulting video, produce structured issues tied to ranges/scenes, and let Talos decide whether and how to repair them.

Minimum issue classes: pacing, duplicate/redundant content, abrupt transition, broken/failed generated asset, subtitle/text obstruction when present, and obvious visual continuity problems detectable by the configured understanding provider.

The loop must terminate under a bounded policy (iteration count/cost/time) and surface unresolved issues instead of retrying indefinitely.

## 8. Jobs, errors and cancellation

- Long-running generation/render/analysis work is represented as explicit jobs with queued/running/completed/failed/cancelled states.
- The UI must remain responsive while jobs run.
- A user can cancel the current high-level operation; cancellation propagates where the underlying provider/runtime supports it.
- Provider timeout, malformed structured output, unavailable service, or generation failure must produce an explicit recoverable state, not an invented success.
- A failed optional generative enhancement should allow a deterministic or simpler fallback where the product result remains valid.

## 9. Privacy, provenance and cost

For each remote media operation Laputa records provider identity, operation type, job/request identity when available, source artifact references, result artifact, and basic cost/usage metadata when the provider exposes it.

API credentials must never be committed to the repository or stored inside project files intended for sharing.

V0.1 does not require a full consumer billing system, but runaway autonomous generation must be bounded by a configurable project/session budget or operation limit.

## 10. Persistence and artifacts

A project must persist:

- creative intent;
- imported asset metadata and original references;
- analysis results sufficient for reuse;
- current story/timeline state;
- generated artifact lineage;
- lock/retain state;
- active/terminal job metadata required for recovery or honest failure reporting.

Caches/proxies may be reconstructable and are not necessarily durable truth.

## 11. V0.1 default UI

Required primary surfaces:

- asset browser/grid;
- central video preview with play/pause/seek;
- scene strip for current ordering and selection;
- one natural-language command input;
- compact creative-intent summary;
- lightweight current-operation/progress feedback;
- export action.

V0.1 must not require a visible multi-track timeline, node graph, model chooser, provider chooser, sampler settings, prompt engineering panel, or large template/style catalog.

## 12. Out of scope

- Mobile application.
- Cloud account/sync/collaboration.
- Automatic social-platform publishing.
- Full professional NLE feature parity.
- Default multi-track editing UI.
- Local deployment of large image/video foundation models.
- Generative music as a required capability.
- Model training/fine-tuning or user-specific LoRA workflows.
- Marketplace/plugin ecosystem.
- Sophisticated provider price arbitrage/routing UI.

## 13. MVP acceptance scenarios

### Scenario A — first cut without workflow choices

Given a project containing a representative fixture of mixed photos and videos  
And functional external providers are configured  
When the user describes a short Vlog in one natural-language request  
Then Laputa produces a playable first-cut preview and ordered scene strip  
And the user is not required to choose a model, provider, node workflow, or professional timeline configuration.

### Scenario B — selection-based revision

Given a playable first cut  
And one scene is selected  
When the user says “make this shorter and more natural”  
Then the resulting edit targets the selected scene by default  
And other locked or unrelated scenes remain unchanged unless the instruction explicitly broadens scope.

### Scenario C — non-destructive generative edit

Given an imported original photo  
When Laputa chooses or the user requests photo animation  
Then any generated video is stored as a derived artifact with source lineage  
And the original photo bytes are unchanged.

### Scenario D — honest asynchronous failure

Given a remote generation job has been submitted  
When the provider fails or times out before producing a valid artifact  
Then Laputa does not insert a nonexistent result into the timeline  
And the user/Agent receives an explicit failure or a documented fallback result.

### Scenario E — reopen and continue

Given the user has created a first cut and at least one revision  
When the application closes and the project is reopened  
Then the latest committed intent, timeline, generated-artifact lineage, and lock state are restored sufficiently to continue editing.

### Scenario F — export

Given a valid current video  
When the user requests export  
Then Laputa produces a playable MP4 in the chosen/inferred supported aspect ratio  
And the export references project state without mutating original assets.

## 14. Initial quality bar

LAPUTA-001 is not complete because individual modules compile. The MVP must expose a user-reachable path from import through first cut, at least one revision, and export. Each implementation Story may deliver a thinner runnable slice, but built-and-unwired capability is partial until a later explicitly tracked integration Story makes it reachable.
