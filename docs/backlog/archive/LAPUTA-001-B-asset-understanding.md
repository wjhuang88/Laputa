# LAPUTA-001-B — Imported assets can be structurally understood

Parent: LAPUTA-001  
Type: Technical/API Story  
Status: Complete  
Closed: 2026-10-20  
Issue: #3  
Decision links: `docs/decisions/ADR-0002-functional-provider-abstraction.md`  
Iteration: `docs/iterations/2026-10-20-asset-understanding-foundation.md`

## Outcome

Imported photos and videos can now be inspected through a provider-neutral media-understanding capability and converted into validated Laputa-owned structured analysis. Valid results are persisted as rebuildable derived state, reused for unchanged source media, and rejected when source provenance no longer matches.

## Delivered

- Canonical `AssetAnalysis`, semantic tags, and validated temporal video segments owned by `laputa-domain`.
- Functional `MediaUnderstandingProvider` contract with provider-neutral failures.
- Ready/failed understanding state under `.laputa/cache/<asset-id>/understanding.json`, keyed to the imported `source_sha256`.
- Provider-independent `inspect_asset` orchestration with cache reuse, retryable failure semantics, and candidate validation before persistence.
- Current-source SHA-256 verification before cache/provider use; externally changed originals are rejected instead of silently reusing stale analysis.
- Source hashing runs on Tokio's blocking pool so large media does not block the async runtime executor.
- Isolated Gemini adapter implementing resumable Files upload, processing polling, Interactions structured JSON output, canonical translation, error mapping, and best-effort remote cleanup.
- Explicit Gemini adapter rejection for Story A containers it does not currently support rather than pretending capability.
- Opt-in ignored live Gemini smoke test requiring explicit credentials and a local fixture.
- Fixed Story A schema-v1 manifest compatibility fixture proving no project-manifest migration is required.

## Acceptance

- [x] Fixture image and video produce validated structured analysis through the functional interface.
- [x] Core domain/tool code does not depend on Gemini SDK/schema/model/resource identifiers.
- [x] Invalid provider output is rejected explicitly and cannot be persisted as successful analysis.
- [x] Repeated inspection of unchanged media reuses persisted Ready analysis without a second provider call.
- [x] Story A project manifests remain openable without manifest migration; `PROJECT_SCHEMA_VERSION` stays at 1.
- [x] Analysis persistence rejects a stale/mismatched expected source revision.
- [x] Inspection rejects an original file that changed after import before invoking the provider or using cached analysis.

## Evidence

- Provider-independent core clean CI `39354940570`: success.
- Read-only full CI after Gemini adapter integration `39356515502`: success.
- Final code validation CI `39379788027`: success (`fmt`, workspace `check`, Clippy with `-D warnings`, workspace tests, desktop-ui test, governance).
- Draft implementation/closure PR: #13.
- Lockfile audit reset to the main baseline before incrementally resolving the Gemini HTTP/TLS dependency graph; unrelated Talos/Wasmtime/Tokio version drift was removed.
- `crates/laputa-media/tests/story_a_manifest_compat.rs` pins the Story A persisted manifest shape.
- `crates/laputa-provider-gemini/tests/live_smoke.rs` provides the external smoke entry point.

## Live-smoke residual

No Gemini credential or user-provided live-media fixture was available in this iteration, so the external live smoke was **not executed** and no live evidence is claimed. It can be run explicitly with:

```sh
GEMINI_API_KEY=... \
LAPUTA_GEMINI_MODEL=... \
LAPUTA_GEMINI_SMOKE_MEDIA=/path/to/media \
cargo test -p laputa-provider-gemini --test live_smoke -- --ignored --nocapture
```

Normal CI remains credential-free and deterministic.

## Residuals intentionally not part of Story B

- Talos First Cut/story generation (`LAPUTA-001-C`).
- Embedding/vector retrieval and richer Character/Location/Event graphs.
- Face-identity resolution or local VLM deployment.
- Automatic proxy/transcode routing for source containers unsupported by the first Gemini adapter.
