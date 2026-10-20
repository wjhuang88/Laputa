# Iteration — Structured Asset Understanding Foundation

Status: planned  
Date: 2026-10-20  
Selected Story: `LAPUTA-001-B`  
Issue: #3

## Goal

Deliver the smallest reusable structured-understanding slice on top of the completed Story A project/asset boundary: canonical Laputa-owned analysis data, a functional `MediaUnderstandingProvider` contract, persisted analysis state keyed to the imported source revision, and a provider-independent inspection path that reuses valid cached analysis.

## Published baseline

`LAPUTA-001-A` is complete: the user confirmed the real desktop import/reopen path behaves as expected, complementing the already-passing automated validation.

This iteration selects only `LAPUTA-001-B`. It does **not** start Talos First Cut, embeddings/vector search, face-identity resolution, local VLM deployment, Scene Strip, generative media, or editing behavior.

Expected technical/user-visible outcome:

`persisted Asset -> inspect through functional provider -> validate canonical analysis -> persist success/failure -> reopen/reinspect -> reuse valid analysis`

No vendor/model selector is added to the consumer GUI.

## Dependency order

1. Close/synchronize `LAPUTA-001-A` documentation and dependency state from the accepted real-desktop smoke.
2. Add a backwards-compatible project-manifest migration path for the new persisted analysis field.
3. Define canonical `AssetAnalysis` / temporal segment / analysis-state types in `laputa-domain`.
4. Define `MediaUnderstandingProvider` in `laputa-provider`; the contract consumes Laputa asset context and returns Laputa-owned candidate analysis rather than a vendor schema.
5. Add `ProjectStore` operations that persist validated ready/failure analysis against an expected source SHA-256 revision.
6. Add a provider-independent inspection coordinator in the composition layer; valid cached analysis must bypass the provider.
7. Add fake-provider contract/orchestration tests for image/video, invalid output rejection, failure persistence, stale-source protection, and cache reuse.
8. Evaluate/implement the first external API adapter as a separate bounded checkpoint; absence of live credentials must not weaken deterministic contract tests.

## Success criteria

- Canonical analysis is owned by Laputa domain types and contains an asset-level summary/tags plus temporal segments for video where applicable.
- Analysis validation rejects empty/invalid structures and invalid video ranges before they can be persisted as ready.
- Project schema evolution can open Story A manifests that do not contain analysis state.
- `MediaUnderstandingProvider` has no vendor/model choice in its consumer-facing call contract.
- Project persistence records ready or explicit failed analysis against the expected `source_sha256`; a stale source revision cannot be silently accepted.
- Repeated inspection of an unchanged asset returns persisted ready analysis without invoking the provider again.
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

- schema-v1 project manifest can open under the new domain schema;
- image analysis can be validated/persisted;
- video temporal segments are ordered, bounded and non-empty;
- malformed candidate analysis is not persisted as `Ready`;
- provider failure is explicit;
- cached ready analysis prevents a second provider call;
- expected source hash mismatch rejects the persistence mutation.

## Risks and assumptions

- The first canonical schema intentionally stays small: summary, tags and temporal segments. Characters/locations/events may be projected later when concrete First Cut requirements justify richer fields.
- `source_sha256` is the Story B source revision key because Story A already persists it and original assets are immutable by contract.
- Persisting analysis inside the project manifest is acceptable for V0.1 structured metadata volume; large model artifacts/raw responses must not be embedded there.
- Vendor raw JSON/SDK types are adapter-local and are never persisted as the canonical project schema.
- A live external API smoke may require credentials unavailable to CI; deterministic fake-adapter coverage remains mandatory.

## Closure

Result: pending.
