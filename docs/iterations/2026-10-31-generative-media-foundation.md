# Iteration — Generative Media Foundation

Status: in progress — E5b final validation  
Date: 2026-10-31  
Selected Story: `LAPUTA-001-E`  
Issue: #6

## Goal

Establish the durable asynchronous generation foundation required for Story E, then prove it through coarse-grained Laputa Tools and one external adapter without leaking vendor/model/workflow concepts into the domain, Tool schema or normal user experience.

This iteration intentionally proceeds while Story D remains open for one independent real-desktop acceptance gate. Story E depends only on LAPUTA-001-A and LAPUTA-001-C, both Complete/Closed. No Story F/G/H behavior is included.

## Published baseline

- planning baseline `main`: `f6b95aadd766502599b8c76dbdc0df695852b989`
- iteration-start governance merged by PR #22: `32416df6cd173a09b5646062aa8b05ff27a8b278`
- E1 durable generation state merged by PR #23: `540ca3286f182b46e880c1e817e92e6b1ec3486f`
- E2 functional generation provider merged by PR #24: `fbd1b06ed43dbc55f3485184074fb734cfccd0e2`
- E3 durable generation orchestration merged by PR #25: `120e68004bde3576ab593c0b8c5eab1693e3f783`
- E4 coarse-grained generation Tools merged by PR #26: `5e8d3cdb7ae9b2e9c1cd8c6221c70206b0905e39`; post-merge main CI `41285644832` success
- E5a Runway adapter merged by PR #27: `9f3662457655a58b9140dbdcb9459d65328a3f90`; post-merge main CI `41289213797` success
- E5b project-owned Runway HTTPS materialization is under PR #28
- LAPUTA-001-A / Issue #2: Complete/Closed
- LAPUTA-001-C / Issue #4: Complete/Closed
- LAPUTA-001-D / Issue #5: product implementation and automated smoke complete; one real-desktop human acceptance residual remains
- ADR-0002: functional provider abstraction is authoritative
- Architecture contract: external generation is `submit -> job -> terminal state -> verified/materialized artifact`; provider completion alone never authorizes timeline mutation

## Selected scope

1. Define generation request/job/artifact domain state and invariants.
2. Persist enough terminal/non-terminal job metadata for honest recovery and failure reporting.
3. Define provider-neutral functional `VideoGenerationProvider` / `ImageGenerationProvider` boundaries.
4. Provide deterministic Fake adapters and state-machine tests.
5. Materialize and independently validate terminal-success output before publishing a derived Artifact.
6. Wire `media.animate_photo` and `media.generate_broll` through coarse-grained Laputa Tools.
7. Add one external adapter for the selected subset and an opt-in live smoke when credentials are available.
8. Ensure temporary external HTTPS output is copied into project-owned storage before the existing E3 publication gate.

## Explicit exclusions

- No user-visible provider/model selector.
- No vendor/model fields in normal Tool schemas.
- No speculative routing framework or price-arbitrage system.
- No local foundation-model deployment.
- No Story F trim/reorder/delete/lock behavior.
- No Story G critic/repair loop.
- No Story H final export closure.
- No mutation of imported original media.

## Implementation order

### E1 — durable job/artifact contract — Complete

E1 established stable generation request/job/artifact identities, source revision lineage, explicit queued/submitted/running/completed/failed/cancelled states, provider metadata as provenance only, guarded Artifact construction and atomic `.laputa/generation.json` persistence.

Key evidence:

- `DerivedArtifact::from_verified_materialization` rejects non-completed jobs and invalid materialization metadata;
- `GenerationState::validate_for` enforces request/job/artifact linkage, source revision consistency and acyclic derived-artifact lineage;
- deterministic tests cover non-completed publication rejection, terminal restart rejection, stale source/path rejection, persistence/reopen and imported-original hash preservation;
- final branch head `3779d6ef21502849f77622e7f6315cd97252a379`: push CI `41166260806`, PR CI `41166264802` success;
- PR #23 squash merged to `540ca3286f182b46e880c1e817e92e6b1ec3486f`; post-merge main CI `41167010539` success.

### E2 — functional provider contract + Fake adapter — Complete

E2 established provider-neutral async `submit / poll / cancel` contracts and deterministic Fake video/image providers covering lifecycle and error behavior without vendor/model/workflow routing fields in core requests.

Key evidence:

- provider statuses explicitly represent Submitted / Running / Completed / Failed / Cancelled;
- stable errors cover unavailable, rejected, malformed response, unknown job and cancellation unsupported;
- deterministic scenarios cover submitted -> running -> completed, failure, cancellation, submission error, malformed completion and completed-with-empty-output rejection;
- final E2 head `0afcaee1518c1484a52c5aeb71a79ecd81bb6ea5`: push CI `41169352895`, PR CI `41169984609` success;
- PR #24 squash merged to `fbd1b06ed43dbc55f3485184074fb734cfccd0e2`; post-merge main CI `41170400201` success.

### E3 — materialization/orchestration — Complete

E3 established the durable publication boundary: submit, persist and resume job state; verify terminal success; materialize into project-owned derivatives; independently FFprobe-validate; compute SHA-256/byte length; only then publish a derived Artifact.

Key evidence:

- `GenerationOrchestrator` persists durable Queued/Submitted/Running/Completed/Failed/Cancelled transitions;
- provider `Completed` is persisted before publication but does not itself authorize an Artifact;
- local materialization writes `.laputa/derivatives/generation/<job-id>/output.mp4` with temporary-file replacement;
- FFprobe independently requires a non-empty visual file, dimensions, positive duration and video codec;
- current Asset/Artifact source bytes are re-hashed immediately before provider submission to prevent stale lineage;
- missing/invalid output, failure and cancellation never publish an Artifact and never mutate imported originals;
- final PR head `f93c9e86d981aa75cb798e28f2408431f54262f5`: push CI `41173132893`, PR CI `41173151343` success;
- PR #25 squash merged to `120e68004bde3576ab593c0b8c5eab1693e3f783`; post-merge main CI `41183800760` success.

### E4 — coarse-grained Tool path — Complete

E4 wires real Talos Tools through E3:

- `media.animate_photo` takes only an image `asset_id` and product-level `instruction`;
- `media.generate_broll` takes only a product-level `instruction`;
- both Tools depend only on `VideoGenerationProvider`, execute the existing E3 durable generation/materialization/publication pipeline and return product/domain Artifact information;
- public JSON schemas use `additionalProperties: false` and regression-test that `provider`, `vendor`, `model`, `workflow` and `sampler` do not appear;
- deterministic Tool tests use a Fake provider plus a real FFmpeg fixture and prove verified Artifact publication while preserving imported original bytes and request/source lineage.

Evidence:

- formatted implementation head `590b19705b42f03a5813cdef71763b87ed242089`: push CI `41184693535` success;
- docs-synchronized head `6aca25c03ed6f0e7bbb89deb22515bb7add5254c`: push CI `41185349439` success;
- PR #26 squash merged to `5e8d3cdb7ae9b2e9c1cd8c6221c70206b0905e39`;
- post-merge main CI `41285644832` success.

### E5a — Runway external adapter — Complete

E5a adds the first real external video-generation adapter behind the frozen provider-neutral contract:

- independent `laputa-provider-runway` crate;
- Runway HTTP shapes, model/ratio/API-version defaults and secret handling remain inside the adapter;
- zero-source requests map to text-to-video; one image source maps to image-to-video through ephemeral upload;
- Runway PENDING/THROTTLED/RUNNING/SUCCEEDED/FAILED/CANCELED states map to the existing lifecycle;
- successful raw provider output must be HTTPS, but is not treated as a Laputa Artifact;
- deterministic request/payload/status/upload tests cover adapter behavior;
- ignored live smoke reads only `RUNWAYML_API_SECRET`; no credential is committed and no live success is claimed when unavailable.

Evidence:

- pre-PR adapter head `7a75c8a83e70bbe4469e0c68c6a5a64640cfb8de`: push CI `41287150353` success;
- PR #27 squash merged to `9f3662457655a58b9140dbdcb9459d65328a3f90`;
- post-merge main CI `41289213797` success.

### E5b — project-owned HTTPS output materialization — In Review

PR #28 closes the remaining external-output eligibility gap without changing E3/E4 architecture:

- `RunwayProjectVideoGenerationProvider` wraps the core Runway provider while implementing the same `VideoGenerationProvider` contract;
- public construction is available through `new(inner, project_root)` and deterministic tests can inject a `RunwayOutputFetcher` via `with_fetcher`;
- a completed HTTPS locator is downloaded into `.laputa/provider-staging/runway/<task-id>/output.mp4` before being returned to E3;
- staged files are reusable across repeated polling/resume, task ids are traversal-safe, downloads are bounded, empty/insecure output is rejected, and temp-file replacement prevents partially published staging output;
- repeated cancellation treats an already-unknown remote job as idempotent Cancelled;
- E3 remains responsible for copying staging output to final `.laputa/derivatives/...`, independent FFprobe validation, SHA-256/byte length calculation and `DerivedArtifact` publication;
- deterministic cross-boundary coverage injects a fixture fetcher (no real network), passes the project wrapper directly into `GenerateBrollTool`, and verifies project staging -> final derivative -> FFprobe-gated Artifact publication, including that the staging file itself is never the published Artifact.

Prior implementation head `6b9ec1808a66c0d24fcafb6a2ad913a04ff162da` passed exact-head push CI `41290501640` and PR CI `41290506425`. The composition test and synchronized governance docs are being validated on the final PR #28 head before Ready/merge.

## Validation plan

Minimum deterministic validation:

- real FFmpeg preview fixture
- `cargo fmt --all -- --check`
- `cargo check --workspace --locked`
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
- `cargo test --workspace --locked`
- desktop tests
- `scripts/validate_project_governance.sh .`
- Fake-adapter lifecycle/error tests
- original-media hash unchanged across success/failure/cancel paths
- persistence/reopen evidence for durable generation state
- materialization/validation evidence proving missing/invalid output never becomes eligible
- Tool schema regression proving no provider/model/workflow selector leaks
- deterministic Runway project-wrapper -> Tool -> E3 publication composition without real network

## Acceptance evidence required before Story E closure

- A real Laputa Tool can request a derived generation without naming a vendor in its public schema.
- Non-terminal/failed/cancelled jobs are impossible to mistake for completed Artifacts.
- Completed Artifacts preserve source/request/job lineage.
- A completed provider response is independently materialized/validated before eligibility.
- Failure/cancellation leaves original media unchanged and returns an explicit failure or documented safe fallback.
- At least `media.animate_photo` plus one transition/B-roll path is wired through the product Tool surface.
- One real external adapter fits the provider-neutral lifecycle without requiring provider/model/workflow selectors in normal Tool/user flow.
- External temporary HTTPS output is first made project-owned and then traverses the existing E3 validation/publication gate.

## Live external evidence residual

The Runway live smoke remains explicitly opt-in and ignored by default. It reads only `RUNWAYML_API_SECRET`. If no credential is available at closure, deterministic adapter/composition/materialization evidence is the acceptance basis; no live execution is claimed.

## Risks and confirmed boundaries

- **Confirmed:** Story D is not a dependency of E; A and C are the declared dependencies and are Complete.
- **Confirmed:** provider identity belongs in adapter/provenance metadata, not the core functional contract or Tool schema.
- **Confirmed:** provider Completed and timeline-eligible Artifact are distinct states; project-owned materialization and independent validation can fail after remote completion without publishing an Artifact.
- **Confirmed by E4:** product-facing generation requests remain vendor/model/workflow-neutral through the durable provider lifecycle.
- **Confirmed by E5a/E5b:** the selected Runway subset fits the common async lifecycle and can be composed through project-owned staging into the existing E3 publication boundary without introducing a routing framework or user-visible selector.

## Parallel Story D disposition

The historical `2026-10-23-workspace-visual-baseline` iteration is closed as complete for its visual-baseline goal. Story D itself remains open in Issue #5 only for final real-desktop human interaction acceptance. E work must not modify or reinterpret that acceptance gate.
