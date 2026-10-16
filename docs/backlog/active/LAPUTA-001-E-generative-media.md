# LAPUTA-001-E — Talos can request generative derived media without vendor coupling

Parent: LAPUTA-001  
Type: Technical/API Story  
Status: Proposed  
Dependencies: LAPUTA-001-A, LAPUTA-001-C  
Decision links: `docs/decisions/ADR-0002-functional-provider-abstraction.md`

To support photo animation, generated transitions and missing visual material, Laputa needs asynchronous generative media capabilities that preserve source lineage and do not leak vendor contracts into Tools/domain code.

## Scope

- Functional image/video generation contracts required by V0.1.
- One external API adapter for the selected capability subset.
- Explicit generation job state and cancellation/error handling.
- Derived artifact lineage, materialization and timeline eligibility only after verified completion.
- At least `media.animate_photo` and one transition/B-roll path wired through coarse-grained Tools.

## Out of scope

- Local foundation-model deployment, user-visible provider/model selection, sophisticated routing optimization.

## Acceptance

- [ ] A Tool can request a derived generation without naming a vendor in its public schema.
- [ ] Submitted/running/failed jobs cannot be mistaken for completed artifacts.
- [ ] Generated outputs preserve source/request/job lineage.
- [ ] Failure can return an explicit failure or safe deterministic fallback without mutating originals.

## Minimum validation

Fake-adapter state-machine tests, original-media hash test, and opt-in live API smoke test when credentials exist.
