# LAPUTA-001-E — Talos can request generative derived media without vendor coupling

Parent: LAPUTA-001  
Type: Technical/API Story  
Status: In Progress — E5b validation  
Dependencies: LAPUTA-001-A, LAPUTA-001-C（both Complete）  
Decision links: `docs/decisions/ADR-0002-functional-provider-abstraction.md`  
Implementation iteration: `docs/iterations/2026-10-31-generative-media-foundation.md`

To support photo animation, generated transitions and missing visual material, Laputa needs asynchronous generative media capabilities that preserve source lineage and do not leak vendor contracts into Tools/domain code.

## Scope

- Functional image/video generation contracts required by V0.1.
- One external API adapter for the selected capability subset.
- Explicit generation job state and cancellation/error handling.
- Derived artifact lineage, materialization and timeline eligibility only after verified completion.
- At least `media.animate_photo` and one transition/B-roll path wired through coarse-grained Tools.

## Architecture constraints

- Core contracts are functional (`VideoGenerationProvider` / `ImageGenerationProvider` / `ArtifactStore`), never vendor-named.
- External generation follows `submit -> durable job state -> terminal state -> verified/materialized artifact`.
- Submission success is never sufficient to mutate FilmState/Timeline or advertise a usable artifact.
- Imported original media remains byte-for-byte immutable.
- `media.animate_photo` is a product intent, not a forced image-to-video implementation; deterministic motion or no generated motion remains legal when appropriate.
- Normal users and model-facing Tool schemas do not expose Provider, model, workflow, sampler or vendor-specific generation controls.

## Acceptance

- [ ] A Tool can request a derived generation without naming a vendor in its public schema.
- [ ] Submitted/running/failed/cancelled jobs cannot be mistaken for completed artifacts.
- [ ] Generated outputs preserve source/request/job lineage and provider operation metadata required by the V0.1 provenance contract.
- [ ] Only terminal-success artifacts that pass materialization/validation are eligible for timeline use.
- [ ] Failure can return an explicit failure or safe deterministic fallback without mutating originals.
- [ ] Cancellation has an explicit durable state and does not fabricate a result artifact.

## Minimum validation

- Fake-adapter state-machine tests covering submit/running/completed/failed/cancelled and malformed completion.
- Artifact materialization/validation tests proving only verified terminal success becomes eligible.
- Original-media hash test across success, failure and cancellation paths.
- Public Tool schema regression proving no vendor/model selector leaks.
- Opt-in live API smoke test when credentials exist; absence of credentials is recorded as a residual rather than fabricated evidence.

## Implementation slices

1. **E1 — Domain/job/artifact contract — Complete**: generation request identity, durable job states, artifact lineage/eligibility invariants and persistence boundary. PR #23 merged to `540ca3286f182b46e880c1e817e92e6b1ec3486f`.
2. **E2 — Functional provider contract + Fake adapter — Complete**: capability-shaped provider API with deterministic state-machine tests. PR #24 merged to `fbd1b06ed43dbc55f3485184074fb734cfccd0e2`.
3. **E3 — Materialization/orchestration — Complete**: completed provider output is copied into project-owned derivatives, independently FFprobe-validated, hashed and only then published as an Artifact. PR #25 merged to `120e68004bde3576ab593c0b8c5eab1693e3f783`.
4. **E4 — Coarse-grained Tool path — Complete**: `media.animate_photo` and `media.generate_broll` execute the E3 pipeline while keeping public Tool schemas free of provider/vendor/model/workflow/sampler controls. PR #26 merged to `5e8d3cdb7ae9b2e9c1cd8c6221c70206b0905e39`.
5. **E5a — Runway external adapter — Complete**: isolated `laputa-provider-runway` adapter supports text-to-video and image-to-video behind the provider-neutral contract; live smoke remains opt-in via `RUNWAYML_API_SECRET`. PR #27 merged to `9f3662457655a58b9140dbdcb9459d65328a3f90`; post-merge main CI `41289213797` succeeded.
6. **E5b — project-owned HTTPS materialization — In Review**: PR #28 wraps the Runway provider so successful HTTPS output is downloaded into `.laputa/provider-staging/runway/<task-id>/output.mp4` before the existing E3 local materialization and independent FFprobe publication gate. Deterministic cross-boundary coverage composes the project wrapper directly with `GenerateBrollTool` and verifies staging -> final derivative -> Artifact publication without real network access.

Story E remains incomplete until PR #28 exact-head CI, review/merge gates and post-merge main CI are green. Only then are the acceptance checkboxes closed and the Story moved to `archive/`.

## Live external evidence residual

The Runway live smoke is intentionally `#[ignore]` and reads credentials only from `RUNWAYML_API_SECRET`. No credential is committed or persisted. If no credential is available during Story closure, no live success is claimed; deterministic adapter, composition, materialization and publication evidence remains the acceptance basis allowed by this Story.
