# LAPUTA-001-E — Talos can request generative derived media without vendor coupling

Parent: LAPUTA-001  
Type: Technical/API Story  
Status: Ready  
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

## Initial implementation slices

1. **E1 — Domain/job/artifact contract**: generation request identity, durable job states, artifact lineage/eligibility invariants and persistence boundary.
2. **E2 — Functional provider contract + Fake adapter**: capability-shaped provider API with deterministic state-machine tests.
3. **E3 — Materialization/orchestration**: verify completed remote output before publishing a derived Artifact; no Timeline mutation from non-terminal jobs.
4. **E4 — Coarse-grained Tool path**: wire `media.animate_photo` plus one transition/B-roll route without vendor fields in Tool schema.
5. **E5 — External adapter/live residual**: add one selected external adapter and opt-in live smoke without committing credentials.

The iteration may checkpoint these slices independently, but Story E remains incomplete until a runnable Tool path and the acceptance evidence above are present.
