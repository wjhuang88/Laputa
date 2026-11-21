# Iteration — Generation State Concurrency Repair

Status: complete for core Ubuntu-validated defect on governance merge (portability residual #44)  
Date: 2026-11-19  
Selected Story: `LAPUTA-001-I` (Ready technical corrective Story)  
Issue: [#40](https://github.com/wjhuang88/Laputa/issues/40)  
Published baseline: main `ea711169b820bc14da5416734e3f1e7ce7eb07a0`  
Dependency: LAPUTA-001-E historically Complete; new corrective follow-up does not reopen #6

## Goal

Eliminate lost updates of durable generation jobs and artifacts when independently polling/submitting/cancelling within one project, without weakening provider provenance, completion gating, or original-media immutability.

## Change-control / parallelism

The 2026-11-16 workspace visual rework and F3b iteration(s) remain separately non-terminal, with their prior state unchanged. This corrective Story is independent of GUI/Story F and is prioritized because persisted external generation handles may otherwise be lost. No scope expansion to provider routing, UI, or Story E historical acceptance.

## Implementation slices

1. `GenerationStore`: stable sibling lockfile; OS-level exclusive lock and transactional load/mutate/validate/atomic persist. Distinct temp files and no remove-destination fallback on replacement failure. No added external locking dependency (Rust 1.95 `File::lock`).
2. `GenerationOrchestrator`: migrate queue, transition, failure, cancellation, and verified artifact publication to single-job updates against current on-disk state; do not hold lock across remote calls or materialization. Guard stale/terminal status.
3. Deterministic regression: staged overlapping A/B polling and submission, independent submissions, concurrent transitions/publication, process-level writers, rollback/failed writes and imported-original hash preservation.

## Constraints / risk

- Preserve `GenerationJobState` transition legality, source lineage and provider-neutral Tool contracts.
- Keep `.laputa/generation.json` backwards readable and project-owned.
- Lockfile is permanent: deleting or replacing it while operations run is unsafe. File locking relies on a local filesystem honoring OS locks; exotic network filesystems require separate validation.
- Never claim a successful Runway live smoke without credentials. Existing Story E closure remains historically complete.
- External provider completion is not an Artifact until independently materialized and validated.
- Cross-platform behavior, especially Windows rename semantics and concurrent readers, must be evidenced by CI before closure.

## Validation plan

- `cargo fmt --all -- --check`
- `cargo check --workspace --locked`
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
- `cargo test --workspace --locked`
- `scripts/validate_project_governance.sh .`
- Deterministic interleaving + cross-process tests and unchanged-original hashes.
- Exact-head push + PR CI, recorded maintainer self-review (clearly not independent approval), then post-merge CI.

## Evidence / disposition

The fix was merged through [PR #42](https://github.com/wjhuang88/Laputa/pull/42) at `0dfe62d94da6f95f83f7caab3f1bd29cc31d6c65` (implementation head `de49fbb78d68eed6facf0736f6ed9c40d3c24005`). Tests exercise a polling A plus newly submitted B, all committed durable provider handles, independent threaded/process writers, stale terminal transitions, artifact publication, immutable source hashes, and injected persistence failure.

Actual automated evidence:
- Push CI [#44457205819](https://github.com/wjhuang88/Laputa/actions/runs/44457205819): success at exact implementation head (fmt, check, clippy, full workspace tests, desktop, FFmpeg and governance validation).
- PR CI [#44457211375](https://github.com/wjhuang88/Laputa/actions/runs/44457211375): attempt 2 success at the same head. Attempt 1 encountered unrelated Linux `ETXTBSY` in preview decoder fixture, tracked as [#43](https://github.com/wjhuang88/Laputa/issues/43).
- Main code merge CI [#44863056584](https://github.com/wjhuang88/Laputa/actions/runs/44863056584): **must be verified successful before this closeout PR is merged**; do not assume status from the link alone.
- Single-maintainer [self-review #5969243846](https://github.com/wjhuang88/Laputa/pull/42#pullrequestreview-5969243846): formal COMMENT review, **not** independent approval.
- Real Runway live smoke: optional, UNRUN; no credentials or costs asserted.

Closure boundary: the core lost-update correctness defect is verified on the current Ubuntu runner. macOS/Windows locking, replacement and reopen semantics were not tested; tracked in [#44](https://github.com/wjhuang88/Laputa/issues/44), not accepted by this iteration. Closure of this iteration for the exercised platform does not imply all-platform correctness. Other active D/F iterations are unchanged.
