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

### E2 — functional provider contract + Fake adapter — In Progress

Add capability-shaped provider contracts and deterministic Fake behavior for:

- successful asynchronous completion;
- running/poll progression;
- failure;
- cancellation;
- malformed/invalid completion;
- provider claims success but returned output reference is absent/invalid.

Success rule: provider submission success is observable as job state only, never as artifact success.

#### Current E2 checkpoint

Implementation branch: `feature/laputa-001-e2-generation-provider`, based on E1 merge `540ca3286f182b46e880c1e817e92e6b1ec3486f`.

Implemented and pushed:

- provider-neutral `VideoGenerationProvider` and `ImageGenerationProvider` with async `submit / poll / cancel` lifecycle;
- provider-neutral media inputs and generation requests containing semantic instruction + source media only, with no vendor/model/workflow selection fields;
- `GenerationProviderHandle` carries provider/request/job identity only as provenance metadata;
- explicit provider status `Submitted / Running / Completed / Failed / Cancelled`;
- stable error kinds for unavailable, rejected, malformed response, unknown job and cancellation unsupported;
- deterministic Fake Video/Image providers with scripted lifecycle;
- Fake coverage for submitted -> running -> completed, explicit failure, cancellation, submission error, malformed completion and completed-with-empty-output rejection;
- Fake async tests use a dependency-free single-poll executor because Fake methods intentionally perform no asynchronous I/O; real adapter/runtime async behavior remains later scope.

Validation history:

- first E2 CI exposed a `--locked` failure caused only by adding an unnecessary test `tokio` dependency; the dependency was removed instead of creating Cargo.lock churn;
- next CI exposed only rustfmt differences in Fake tests; those exact formatting changes were applied without semantic changes;
- final code head `8006c1edc30d00344ca93acc88a394a94f7f8b78`: push CI `41168599043` **success**, covering FFmpeg fixture, fmt, workspace check, all-target/all-feature Clippy, workspace tests, desktop tests and governance.

E2 remains incomplete until this documentation checkpoint, PR-triggered CI, review and post-merge main CI are green. E3 must not begin before that closure.

### E3 — materialization/orchestration

Implement the boundary that:

1. submits through the functional provider;
2. persists/updates job state;
3. waits/polls or resumes according to the provider contract;
4. verifies terminal success;
5. materializes the result into project-owned derived storage;
6. validates the materialized artifact;
7. only then publishes it as timeline-eligible.

The orchestration may live in Tool code unless independent reuse/lifecycle evidence requires a separate service layer.

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
- **Assumption requiring validation:** one common asynchronous lifecycle shape is sufficient for the selected image/video generation subset. If real adapter semantics materially conflict, record a Spike/ADR update rather than forcing incompatible lifecycle behavior into one interface.
- **Assumption requiring validation:** the first external adapter can support at least one selected E capability without requiring user-visible model/workflow selection.

## Parallel Story D disposition

The historical `2026-10-23-workspace-visual-baseline` iteration is closed as complete for its visual-baseline goal. Story D itself remains open in Issue #5 only for final real-desktop human interaction acceptance. E work must not modify or reinterpret that acceptance gate.
