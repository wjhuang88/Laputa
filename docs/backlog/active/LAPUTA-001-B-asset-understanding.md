# LAPUTA-001-B — Imported assets can be structurally understood

Parent: LAPUTA-001  
Type: Technical/API Story  
Status: Proposed  
Dependencies: LAPUTA-001-A  
Decision links: `docs/decisions/ADR-0002-functional-provider-abstraction.md`

To let the Agent select and reason about media, the system needs reusable structured analysis behind a functional understanding-provider boundary, so that Talos does not repeatedly inspect raw media or depend on one vendor schema.

## Scope

- `MediaUnderstandingProvider` contract and one external-API adapter sufficient for fixtures.
- Structured asset/shot analysis with timestamps/ranges where applicable.
- Persistence/cache of successful analysis linked to source asset revision.
- Validation of malformed/partial provider output and explicit failure state.

## Out of scope

- Embedding/vector search, face-recognition identity service, local VLM deployment.

## Acceptance

- [ ] A fixture image and video can produce validated structured analysis through the functional interface.
- [ ] Core domain/tool code does not depend on the vendor SDK/schema directly.
- [ ] Invalid provider output is rejected explicitly and not persisted as successful analysis.
- [ ] Repeated inspection can reuse valid persisted analysis without mandatory re-upload.

## Minimum validation

Provider contract tests with fake adapter plus one opt-in live integration smoke test when credentials are available.
