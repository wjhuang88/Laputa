# LAPUTA-001-E — Talos can request generative derived media without vendor coupling

Parent: LAPUTA-001  
Type: Technical/API Story  
Status: Complete  
Closed: 2026-11-04  
Issue: #6  
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

- [x] A Tool can request a derived generation without naming a vendor in its public schema.
- [x] Submitted/running/failed/cancelled jobs cannot be mistaken for completed artifacts.
- [x] Generated outputs preserve source/request/job lineage and provider operation metadata required by the V0.1 provenance contract.
- [x] Only terminal-success artifacts that pass materialization/validation are eligible for timeline use.
- [x] Failure can return an explicit failure or safe deterministic fallback without mutating originals.
- [x] Cancellation has an explicit durable state and does not fabricate a result artifact.

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
6. **E5b — project-owned HTTPS materialization — Complete**: PR #28 squash merged to `9a677c9d9c5cd1df8763a7b384bbf1704216bd3a`. The Runway project wrapper stages temporary HTTPS output, then reuses E3's independent local materialization, FFprobe gate and Artifact publication. A deterministic fixture-fetcher test composes the wrapper with the real `GenerateBrollTool` → E3 path without network. Exact-head push CI `41647519486` and PR CI `41647530163` succeeded; post-merge main CI `41649040890` succeeded.

Story E is Complete: PR #28 passed exact-head push/PR CI and the post-merge main validation gate, with no unresolved review threads. All six acceptance criteria are met through deterministic and project-owned publication evidence.

## Closure evidence

- E1/E2/E3/E4/E5a: PRs #23–#27 merged with their documented post-merge evidence.
- E5b: [PR #28](https://github.com/wjhuang88/Laputa/pull/28) squash merged to `9a677c9d9c5cd1df8763a7b384bbf1704216bd3a` after exact-head push CI `41647519486` and PR CI `41647530163` both succeeded.
- Post-merge main CI [`41649040890`](https://github.com/wjhuang88/Laputa/actions/runs/41649040890) succeeded, covering real FFmpeg fixture, fmt, workspace check, all-target/all-feature Clippy, tests, desktop and governance.
- The deterministic Runway fixture-fetcher → `RunwayProjectVideoGenerationProvider` → `GenerateBrollTool` → E3 test proves staging does not count as an Artifact and confirms project-relative derivative publication with matching bytes/SHA-256 after the FFprobe gate.
- Runtime/app does not yet select a configured Runway provider by default; the public composition API is proven at the real Tool boundary and no vendor selector or routing framework was introduced.

## Live external evidence residual

The Runway live smoke is intentionally `#[ignore]` and reads credentials only from `RUNWAYML_API_SECRET`. No credential is committed or persisted. If no credential is available during Story closure, no live success is claimed; deterministic adapter, composition, materialization and publication evidence remains the acceptance basis allowed by this Story.
