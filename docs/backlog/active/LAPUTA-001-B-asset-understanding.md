# LAPUTA-001-B — Imported assets can be structurally understood

Parent: LAPUTA-001  
Type: Technical/API Story  
Status: In Progress  
Dependencies: LAPUTA-001-A (Complete)  
Decision links: `docs/decisions/ADR-0002-functional-provider-abstraction.md`  
Iteration: `docs/iterations/2026-10-20-asset-understanding-foundation.md`

To let the Agent select and reason about media, the system needs reusable structured analysis behind a functional understanding-provider boundary, so that Talos does not repeatedly inspect raw media or depend on one vendor schema.

## Scope

- `MediaUnderstandingProvider` functional contract.
- Canonical Laputa-owned asset analysis: asset-level summary/tags plus temporal segments where applicable.
- One external-API adapter sufficient for fixture/live-smoke validation, isolated from the canonical domain schema.
- Persistence/cache of successful analysis linked to the asset's `source_sha256` revision.
- Explicit failed analysis state; malformed/partial provider output must never be persisted as successful analysis.
- Cache reuse for unchanged assets without mandatory re-upload/provider invocation.

## Out of scope

- Embedding/vector search.
- Face-recognition identity service.
- Local VLM deployment.
- Talos First Cut/story generation.
- Rich Character/Location/Event graph beyond the minimum canonical analysis needed for this Story.

## Acceptance

- [ ] A fixture image and video can produce validated structured analysis through the functional interface.
- [ ] Core domain/tool code does not depend on the vendor SDK/schema directly.
- [ ] Invalid provider output is rejected explicitly and not persisted as successful analysis.
- [ ] Repeated inspection can reuse valid persisted analysis without mandatory re-upload/provider invocation.
- [ ] Story A project manifests without analysis state remain openable after schema evolution.
- [ ] Analysis persistence rejects a stale/mismatched source revision.

## Canonical boundary

The first canonical schema intentionally stays small and provider-neutral:

- asset summary;
- normalized semantic tags;
- temporal segments for videos, each with a validated time range, summary and tags;
- persisted ready/failed state keyed to `source_sha256`.

Vendor raw JSON, SDK request/response objects, model names and upload identifiers are adapter-local implementation details and are not canonical project state.

## Minimum validation

- Fake provider contract/orchestration tests for image and video.
- Invalid/malformed candidate analysis rejection tests.
- Cache hit test proving a second inspect does not call the provider.
- Source-revision mismatch test.
- Project-schema backwards-compatibility test from the Story A manifest shape.
- One opt-in live external-adapter smoke test when credentials are available; lack of credentials is recorded explicitly and never replaced by fabricated evidence.

## Implementation order

1. Domain analysis types + backwards-compatible project schema evolution.
2. Functional provider contract.
3. ProjectStore ready/failed analysis persistence.
4. Provider-independent inspection orchestration + deterministic fake-provider tests.
5. External adapter checkpoint and optional live smoke.

## State owners to synchronize

- `docs/backlog/PRODUCT-BACKLOG.md`
- parent `LAPUTA-001` summary
- `docs/iterations/2026-10-20-asset-understanding-foundation.md`
- Issue #3
