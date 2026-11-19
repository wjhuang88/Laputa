# Iteration — Generation State Concurrency Repair

Status: planned (code not yet changed)  
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

Planning snapshot only. Implementation, automated runtime tests and CI are initially UNRUN. Final status must be `complete`, `partial`, or `blocked` with actual run URLs.
