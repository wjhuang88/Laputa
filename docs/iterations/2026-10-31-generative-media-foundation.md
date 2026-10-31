# Iteration — Generative Media Foundation

Status: in progress  
Date: 2026-10-31  
Selected Story: `LAPUTA-001-E`  
Issue: #6

## Goal

Establish the durable asynchronous generation foundation required for Story E, then prove it through a coarse-grained Laputa Tool path without leaking vendor/model/workflow concepts into the domain, Tool schema or normal user experience.

This iteration intentionally starts while Story D remains open for one independent real-desktop acceptance gate. Story E is allowed to proceed because its declared dependencies are only LAPUTA-001-A and LAPUTA-001-C, both Complete/Closed. No Story F/G/H behavior is pulled into this batch.

## Published baseline

- planning baseline `main`: `f6b95aadd766502599b8c76dbdc0df695852b989`
- iteration-start governance merged by PR #22: `32416df6cd173a09b5646062aa8b05ff27a8b278`
- E1 durable generation state merged by PR #23: `540ca3286f182b46e880c1e817e92e6b1ec3486f`
- E2 functional generation provider merged by PR #24: `fbd1b06ed43dbc55f3485184074fb734cfccd0e2`
- LAPUTA-001-A / Issue #2: Complete/Closed
- LAPUTA-001-C / Issue #4: Complete/Closed
- LAPUTA-001-D / Issue #5: product implementation and automated smoke complete; one real-desktop human acceptance residual remains
- ADR-0002: functional provider abstraction is authoritative
- Architecture contract: external generation is `submit -> job -> terminal state -> verified/materialized artifact`; submission alone never authorizes timeline mutation

## Selected scope

1. Define generation request/job/artifact domain state and invariants.
2. Persist enough terminal/non-terminal job metadata for honest recovery and failure reporting.
3. Define functional `VideoGenerationProvider` / `ImageGenerationProvider` boundary needed by the selected capability path, without vendor-named core interfaces.
4. Provide a deterministic Fake adapter and state-machine tests.
5. Materialize and validate terminal-success output before publishing a derived Artifact.
6. Wire `media.animate_photo` plus one transition/B-roll path through coarse-grained Laputa Tools.
7. Add one external adapter for the selected subset and an opt-in live smoke when credentials are available.

## Explicit exclusions

- No user-visible Provider/model selector.
- No vendor/model fields in normal Tool schemas.
- No speculative routing framework or price-arbitrage system.
- No local foundation-model deployment.
- No Story F trim/reorder/delete/lock behavior.
- No Story G critic/repair loop.
- No Story H final export closure.
- No mutation of imported original media.

## Implementation order

### E1 — durable job/artifact contract — Complete

Define the smallest domain structures required to represent:

- stable generation request identity;
- source references and source hashes;
- operation/capability identity;
- queued/submitted/running/completed/failed/cancelled state;
- provider request/job metadata as provenance metadata rather than core routing input;
- a derived Artifact identity and materialized location;
- lineage from Artifact -> generation job -> request -> source asset/artifact.

Success rule: no non-terminal or failed/cancelled state can be converted into a timeline-eligible Artifact.

#### E1 evidence

- `laputa-domain` owns stable `GenerationRequestId`, `GenerationJobId` and `ArtifactId` identities;
- product-level generation operations and Asset/Artifact source revisions are persisted without vendor routing fields;
- job lifecycle explicitly represents queued/submitted/running/completed/failed/cancelled with guarded transitions;
- provider/request/job identifiers are retained only as provenance metadata;
- `GenerationJob` state/request and `DerivedArtifact` publication fields are sealed behind validated APIs;
- `DerivedArtifact::from_verified_materialization` rejects every non-completed job and validates non-empty project-relative paths, SHA-256 and byte length;
- `GenerationState::validate_for` enforces unique request/job/artifact identities, project/source revision consistency, Artifact -> Job -> Request linkage and acyclic derived-artifact lineage;
- `laputa-media::GenerationStore` persists `.laputa/generation.json` atomically and validates before save and after reopen;
- deterministic tests cover non-completed publication rejection, terminal-state restart rejection, stale source rejection, invalid path rejection, running-job reopen, completed-lineage reopen and original-media hash preservation;
- final branch head `3779d6ef21502849f77622e7f6315cd97252a379`: push CI `41166260806` success, PR CI `41166264802` success;
- PR #23 squash merged to `540ca3286f182b46e880c1e817e92e6b1ec3486f`; post-merge main CI `41167010539` success.

### E2 — functional provider contract + Fake adapter — Complete

Add capability-shaped provider contracts and deterministic Fake behavior for:

- successful asynchronous completion;
- running/poll progression;
- failure;
- cancellation;
- malformed/invalid completion;
- provider claims success but returned output reference is absent/invalid.

Success rule: provider submission success is observable as job state only, never as artifact success.

#### E2 evidence

- provider-neutral `VideoGenerationProvider` and `ImageGenerationProvider` define async `submit / poll / cancel` lifecycle;
- provider-neutral media inputs and generation requests contain semantic instruction + source media only, with no vendor/model/workflow selection fields;
- `GenerationProviderHandle` carries provider/request/job identity only as provenance metadata;
- explicit provider status `Submitted / Running / Completed / Failed / Cancelled`;
- stable error kinds for unavailable, rejected, malformed response, unknown job and cancellation unsupported;
- deterministic Fake Video/Image providers cover submitted -> running -> completed, explicit failure, cancellation, submission error, malformed completion and completed-with-empty-output rejection;
- final E2 head `0afcaee1518c1484a52c5aeb71a79ecd81bb6ea5`: push CI `41169352895` success and PR CI `41169984609` success;
- PR #24 squash merged to `fbd1b06ed43dbc55f3485184074fb734cfccd0e2`; post-merge main CI `41170400201` success.

### E3 — materialization/orchestration — In Review

Implement the boundary that:

1. submits through the functional provider;
2. persists/updates job state;
3. waits/polls or resumes according to the provider contract;
4. verifies terminal success;
5. materializes the result into project-owned derived storage;
6. validates the materialized artifact;
7. only then publishes it as timeline-eligible.

The orchestration is placed in `laputa-agent`, the existing composition layer that already depends on domain, media and provider. `laputa-media` remains provider-independent.

#### Current E3 checkpoint

Implementation branch: `feature/laputa-001-e3-generation-orchestration`, based on E2 merge `fbd1b06ed43dbc55f3485184074fb734cfccd0e2`.

Implemented and validated:

- `GenerationOrchestrator` persists Queued before provider submission, then durable Submitted/Running/Completed/Failed/Cancelled transitions through `GenerationStore`;
- resumable `resume_video_generation` reconstructs provider provenance from durable state; a temporary Provider `Unavailable` polling error leaves the durable non-terminal job resumable rather than falsely failing it;
- explicit cancellation persists terminal cancellation and never publishes an Artifact;
- provider `Completed` is persisted before any Artifact publication, but does not itself authorize an Artifact;
- `GenerationOutputMaterializer` keeps provider-output transport separate from orchestration; the first deterministic implementation copies local/file output into `.laputa/derivatives/generation/<job-id>/output.mp4` using temporary-file replacement;
- `GenerationOutputValidator` independently checks the local result; the first implementation uses FFprobe and requires a non-empty visual file, valid dimensions, positive video duration and a video codec;
- after validation, E3 computes the materialized SHA-256 and byte length and only then enters E1 `DerivedArtifact::from_verified_materialization`;
- provider Completed with missing/invalid local output remains a Completed Job but publishes no Artifact, preserving the distinction between remote job completion and locally eligible derived media;
- Provider inputs are derived directly from durable `GenerationRequest` lineage rather than a second caller-provided source list, preventing request/source drift;
- immediately before provider submission, current Asset and published Artifact bytes are re-hashed and must match the recorded lineage SHA-256, preventing externally modified bytes from being sent under stale lineage;
- original media remains unchanged across success, provider failure, cancellation, malformed completion and missing-output paths;
- deterministic tests cover successful real-FFmpeg video materialization/publication, provider failure, cancellation/reopen, malformed completion, missing output, externally changed imported Asset rejection and externally changed published Artifact rejection;
- an early E3 CI failed only `cargo fmt --check`; formatting was corrected without semantic change;
- the formatted implementation head `3d347b063bf8ac4e8a09ab805f4c93d827212b40` then passed full push CI `41171624761`;
- final source-revision-guard head `f098bc77e04b1dcc57f1ba550ed013fbd19cd449` passed full push CI `41172397650`, including real FFmpeg fixture, fmt, workspace check, all-target/all-feature Clippy, workspace tests, desktop tests and governance.

E3 remains incomplete until PR-triggered CI, review and post-merge main CI are green. E4 must not begin before that closure.

### E4 — coarse-grained Tool path

Wire at least:

- `media.animate_photo`;
- one `media.create_transition` or `media.generate_broll` path.

Tool public schemas describe product intent only. They must not expose vendor, model, sampler, raw workflow or vendor-specific generation parameters.

### E5 — external adapter/live residual

Select one external adapter only after the functional contract is fixed by E1-E4. Credentials remain environment/configuration concerns and are never committed or persisted in shareable project files.

An opt-in live smoke is added when feasible. If no credential is available, deterministic acceptance remains mandatory and the missing live evidence is recorded explicitly as a residual.

## Validation plan

Minimum deterministic validation:

- `cargo fmt --all -- --check`
- `cargo check --workspace --locked`
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
- `cargo test --workspace --locked`
- `scripts/validate_project_governance.sh .`
- Fake-adapter tests for submitted/running/completed/failed/cancelled/malformed completion
- original-media hash unchanged across success/failure/cancel paths
- persistence/reopen test for job metadata required for honest recovery
- artifact validation proving missing/invalid output never becomes eligible
- Tool schema regression proving no provider/model/workflow selector leaks

## Acceptance evidence required before Story E closure

- A real Laputa Tool can request a derived generation without naming a vendor in its public schema.
- Non-terminal/failed/cancelled jobs are impossible to mistake for completed Artifacts.
- Completed Artifacts preserve source/request/job lineage.
- A completed provider response is independently materialized/validated before eligibility.
- Failure/cancellation leaves original media unchanged and returns an explicit failure or documented safe fallback.
- At least `media.animate_photo` plus one transition/B-roll path is wired through the product Tool surface.

## Risks and assumptions

- **Confirmed:** D is not a dependency of E; A and C are the declared dependencies and are Complete.
- **Confirmed:** generation provider identity belongs in adapter/provenance metadata, not the core functional contract or Tool schema.
- **Confirmed:** submitted external jobs cannot be treated as generated artifacts.
- **Confirmed by E3:** provider Completed and timeline-eligible Artifact are separate states; local materialization and validation can fail after remote completion without producing an Artifact.
- **Assumption requiring validation:** one common asynchronous lifecycle shape is sufficient for the selected image/video generation subset. If real adapter semantics materially conflict, record a Spike/ADR update rather than forcing incompatible lifecycle behavior into one interface.
- **Assumption requiring validation:** the first external adapter can support at least one selected E capability without requiring user-visible model/workflow selection.

## Parallel Story D disposition

The historical `2026-10-23-workspace-visual-baseline` iteration is closed as complete for its visual-baseline goal. Story D itself remains open in Issue #5 only for final real-desktop human interaction acceptance. E work must not modify or reinterpret that acceptance gate.
