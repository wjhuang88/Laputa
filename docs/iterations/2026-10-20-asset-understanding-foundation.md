# Iteration — Structured Asset Understanding Foundation

Status: complete  
Date: 2026-10-20  
Selected Story: `LAPUTA-001-B`  
Issue: #3

## Goal

Deliver the smallest reusable structured-understanding slice on top of the completed Story A project/asset boundary: canonical Laputa-owned analysis data, a functional `MediaUnderstandingProvider` contract, derived analysis state keyed to the imported source revision, and a provider-independent inspection path that reuses valid cached analysis.

## Published baseline

`LAPUTA-001-A` is complete: the user confirmed the real desktop import/reopen path behaves as expected, complementing the already-passing automated validation.

This iteration selected only `LAPUTA-001-B`. It did **not** start Talos First Cut, embeddings/vector search, face-identity resolution, local VLM deployment, Scene Strip, generative media, or editing behavior.

Delivered technical flow:

`persisted Asset -> verify source revision -> inspect through functional provider -> validate canonical analysis -> cache success/failure -> reopen/reinspect -> reuse valid analysis`

No vendor/model selector was added to the consumer GUI.

## Storage boundary

Structured understanding is reconstructible derived data, so it does **not** change `project.json` or the Story A Project/Asset manifest contract. The canonical analysis state is persisted at:

`<project>/.laputa/cache/<asset-id>/understanding.json`

The cache record embeds the `source_sha256` that produced it. Inspection also verifies the current original file hash against the imported provenance before consulting cache or provider. A mismatch is rejected explicitly rather than silently accepting stale analysis. Vendor raw responses, upload IDs and model-specific metadata are not persisted as canonical state.

This keeps `PROJECT_SCHEMA_VERSION` at 1 and makes existing Story A projects compatible without a manifest migration.

## Delivered dependency order

1. Synchronized the completed `LAPUTA-001-A` dependency state.
2. Added canonical `AssetAnalysis`, temporal segment and ready/failed analysis-state types to `laputa-domain` without changing `Asset`/`ProjectManifest` persistence shape.
3. Added `MediaUnderstandingProvider` to `laputa-provider`; the contract consumes Laputa asset context and returns Laputa-owned candidate analysis rather than a vendor schema.
4. Added derived-cache persistence for validated ready/failure analysis against an expected source SHA-256 revision in `ProjectStore`.
5. Added provider-independent inspection orchestration in `laputa-agent`; valid cached analysis bypasses the provider.
6. Added deterministic fake-provider tests for image/video, invalid output rejection, provider failure/retry, stale source protection and cache reuse.
7. Added the first external adapter as isolated `laputa-provider-gemini`, using current Gemini Files + Interactions REST shapes and structured JSON output.
8. Added Story A manifest compatibility and opt-in Gemini live-smoke integration tests.
9. Added current-file SHA-256 verification before inspect; hashing runs via Tokio `spawn_blocking` so large files do not block the async executor.
10. Audited the dependency lockfile and removed unrelated version drift before final validation.

## Success criteria

- [x] Canonical analysis is owned by Laputa domain types and contains an asset-level summary/tags plus temporal segments for video where applicable.
- [x] Analysis validation rejects empty/invalid structures and invalid video ranges before they can be persisted as Ready.
- [x] Existing Story A manifests remain unchanged/openable because understanding state is stored outside `project.json`.
- [x] `MediaUnderstandingProvider` has no vendor/model choice in its consumer-facing call contract.
- [x] Derived cache records Ready or explicit Failed analysis against the expected `source_sha256`; stale or externally changed source revisions cannot be silently accepted.
- [x] Repeated inspection of an unchanged asset returns cached Ready analysis without invoking the provider again.
- [x] Fake-provider tests cover both image and video paths plus malformed/failure/cache/revision behavior.
- [x] The first external adapter remains isolated from domain/tool schema.

## Validation evidence

Final code validation run `39379788027` passed the complete read-only CI gate:

```sh
cargo fmt --all -- --check
cargo check --workspace --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked
cargo test -p laputa-app --features desktop-ui --bin laputa-desktop --locked
scripts/validate_project_governance.sh .
```

Additional focused evidence includes:

- fixture image analysis validation/cache path;
- fixture video temporal analysis validation/cache path;
- malformed candidate analysis persisted as Failed, never Ready;
- provider failure explicit and retryable;
- cache hit prevents a second provider call;
- stale expected source hash rejects persistence;
- externally changed original source rejects inspection before cache/provider use;
- Story A schema-v1 project manifest opens without migration;
- Gemini adapter response/MIME/schema translation unit coverage.

Earlier clean checkpoints: core/provider-independent run `39354940570` and read-only adapter integration run `39356515502`.

## External adapter checkpoint

The Gemini adapter owns upload/poll/request/response mechanics and translates only validated canonical `AssetAnalysis` across the functional provider boundary. Model names and Gemini resource URIs remain adapter configuration/state, not project/domain schema.

An opt-in ignored live smoke is provided at `crates/laputa-provider-gemini/tests/live_smoke.rs`. No Gemini credential or user-provided live-media fixture was available during this iteration, so the live external smoke was **not executed** and no live evidence is claimed.

Manual live-smoke command:

```sh
GEMINI_API_KEY=... \
LAPUTA_GEMINI_MODEL=... \
LAPUTA_GEMINI_SMOKE_MEDIA=/path/to/media \
cargo test -p laputa-provider-gemini --test live_smoke -- --ignored --nocapture
```

## Risks and residuals

- The first canonical schema intentionally stays small: summary, tags and temporal segments. Characters/locations/events may be projected later when concrete First Cut requirements justify richer fields.
- Analysis cache is disposable/rebuildable; deleting `.laputa/cache/<asset-id>/understanding.json` does not corrupt `project.json` or imported Asset inventory.
- The first Gemini adapter explicitly rejects imported containers it cannot currently submit rather than changing the Story A import contract; proxy/transcode routing can be added when justified.
- Live external behavior remains a separately executable smoke because normal CI is intentionally credential-free.

## Closure

Result: Complete. `LAPUTA-001-B` is dependency-closed and `LAPUTA-001-C` is unblocked for selection by a new iteration. No First Cut work was started in this iteration.
