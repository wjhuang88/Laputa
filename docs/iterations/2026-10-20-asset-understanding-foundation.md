# Iteration — Structured Asset Understanding Foundation

Status: in progress  
Date: 2026-10-20  
Selected Story: `LAPUTA-001-B`  
Issue: #3

## Goal

Deliver the smallest reusable structured-understanding slice on top of the completed Story A project/asset boundary: canonical Laputa-owned analysis data, a functional `MediaUnderstandingProvider` contract, derived analysis state keyed to the imported source revision, and a provider-independent inspection path that reuses valid cached analysis.

## Published baseline

`LAPUTA-001-A` is complete: the user confirmed the real desktop import/reopen path behaves as expected, complementing the already-passing automated validation.

This iteration selects only `LAPUTA-001-B`. It does **not** start Talos First Cut, embeddings/vector search, face-identity resolution, local VLM deployment, Scene Strip, generative media, or editing behavior.

Expected technical/user-visible outcome:

`persisted Asset -> inspect through functional provider -> validate canonical analysis -> cache success/failure -> reopen/reinspect -> reuse valid analysis`

No vendor/model selector is added to the consumer GUI.

## Storage boundary

Structured understanding is reconstructible derived data, so it does **not** change `project.json` or the Story A Project/Asset manifest contract. The canonical analysis state is persisted at:

`<project>/.laputa/cache/<asset-id>/understanding.json`

The cache record embeds the `source_sha256` that produced it. A mismatch is treated as a cache miss / stale-revision rejection rather than silently reusing analysis. Vendor raw responses, upload IDs and model-specific metadata are not persisted as canonical state.

This keeps `PROJECT_SCHEMA_VERSION` at 1 and makes existing Story A projects compatible without a manifest migration.

## Dependency order

1. Close/synchronize `LAPUTA-001-A` documentation and dependency state from the accepted real-desktop smoke.
2. Define canonical `AssetAnalysis` / temporal segment / analysis-state types in `laputa-domain` without changing `Asset`/`ProjectManifest` persistence shape.
3. Define `MediaUnderstandingProvider` in `laputa-provider`; the contract consumes Laputa asset context and returns Laputa-owned candidate analysis rather than a vendor schema.
4. Add derived-cache persistence for validated ready/failure analysis against an expected source SHA-256 revision.
5. Add a provider-independent inspection coordinator in the composition layer; valid cached analysis must bypass the provider.
6. Add fake-provider contract/orchestration tests for image/video, invalid output rejection, failure persistence, stale-source protection, and cache reuse.
7. Implement the first external API adapter as a separate bounded checkpoint; Gemini is the current candidate because its official API supports image/video inputs plus JSON-Schema structured output.
8. Run deterministic validation; live external smoke remains opt-in when credentials are available.

## Success criteria

- Canonical analysis is owned by Laputa domain types and contains an asset-level summary/tags plus temporal segments for video where applicable.
- Analysis validation rejects empty/invalid structures and invalid video ranges before they can be persisted as ready.
- Existing Story A manifests remain unchanged/openable because understanding state is stored outside `project.json`.
- `MediaUnderstandingProvider` has no vendor/model choice in its consumer-facing call contract.
- Derived cache records ready or explicit failed analysis against the expected `source_sha256`; a stale source revision cannot be silently accepted.
- Repeated inspection of an unchanged asset returns cached ready analysis without invoking the provider again.
- Fake-provider tests cover both image and video paths plus malformed/failure/cache behavior.
- The first external adapter remains isolated from domain/tool schema; if live credentials are unavailable, live smoke is opt-in and recorded as residual rather than fabricated.

## Validation plan

```sh
cargo fmt --all -- --check
cargo check --workspace --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked
scripts/validate_project_governance.sh .
```

Focused tests must additionally prove:

- Story A project manifest stays openable without migration;
- image analysis can be validated/cached;
- video temporal segments are ordered, bounded and non-empty;
- malformed candidate analysis is not persisted as `Ready`;
- provider failure is explicit and retryable;
- cached ready analysis prevents a second provider call;
- expected source hash mismatch rejects the cache mutation.

## External adapter checkpoint

Current official Gemini API evidence supports the intended adapter shape:

- Files API supports reusable uploads for image/video media.
- Video understanding supports temporal inspection and timestamps.
- Interactions API supports response JSON constrained by JSON Schema.

The adapter will own upload/poll/request/response mechanics and translate only the validated canonical `AssetAnalysis` across the functional provider boundary. Model names and Gemini resource URIs remain adapter configuration/state, not project/domain schema.

## Risks and assumptions

- The first canonical schema intentionally stays small: summary, tags and temporal segments. Characters/locations/events may be projected later when concrete First Cut requirements justify richer fields.
- `source_sha256` is the Story B source revision key because Story A already persists it and original assets are immutable by contract.
- Analysis cache is disposable/rebuildable; deleting `.laputa/cache/<asset-id>/understanding.json` must not corrupt `project.json` or imported Asset inventory.
- Vendor raw JSON/SDK types are adapter-local and are never persisted as the canonical project schema.
- A live external API smoke may require credentials unavailable to CI; deterministic fake-adapter coverage remains mandatory.

## Closure

Result: pending.
