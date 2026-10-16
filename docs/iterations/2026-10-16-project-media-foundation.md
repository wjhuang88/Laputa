# Iteration — Project and Media Foundation

Status: partial  
Date: 2026-10-16  
Selected Story: `LAPUTA-001-A`  
Issue: #2

## Goal

Deliver the smallest runnable/testable Laputa foundation that lets the application create a project, import supported local photo/video files without mutating originals, persist the imported asset inventory, and reopen the project.

## Published baseline

This iteration implements only `LAPUTA-001-A`. It does not start AI analysis, Talos first-cut behavior, generation, timeline editing, or provider integration.

Expected user-visible path at closure:

`create project -> import local media -> inspect imported asset inventory -> close -> reopen -> same inventory restored`

## Dependency order

1. Define project/asset domain state and serialization.
2. Define project-local storage layout for durable metadata and reconstructable derivatives.
3. Implement non-destructive importer and media probe boundary.
4. Wire a minimal application entrypoint/CLI flow that exercises create/import/open.
5. Add hash-based tests proving originals are unchanged.
6. Run workspace/governance validation and record evidence.

## Delivered foundation

- Versioned `ProjectManifest`, `ProjectId`, `AssetId`, media kind and technical metadata.
- `.laputa/project.json`, cache and derivatives project layout.
- Batch-safe non-destructive importer that references canonical original paths and records SHA-256 source provenance.
- `MediaProbe` abstraction plus deterministic `ffprobe` adapter.
- Developer CLI for project create/import/open.
- Reopen restores persisted imported inventory.
- Failure paths for unsupported media, duplicate input and probe failure do not commit partial manifest state.

## Actual validation evidence

GitHub Actions run `38860507105` completed successfully on Rust 1.95.

Passed commands:

```sh
cargo fmt --all -- --check
cargo check --workspace --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked
scripts/validate_project_governance.sh .
```

Focused test evidence:

- `import_preserves_original_bytes_and_reopens_inventory` — passed.
- `invalid_batch_does_not_partially_mutate_manifest` — passed.
- `duplicate_import_is_rejected_without_manifest_change` — passed.
- `probe_failure_does_not_mutate_manifest_or_original` — passed.
- `laputa-domain` project-schema test — passed.

The CI log ended with `Laputa governance validation passed`.

## Residual work

This iteration is intentionally closed as `partial` relative to the selected Story because the Story's user-facing desktop acceptance is not yet complete:

- GPUI file picker and/or drag-and-drop import path remains.
- Asset Browser rendering remains.
- Manual desktop import/reopen smoke evidence remains.

The core persistence/import foundation is complete and suitable for merge as a checkpoint. `LAPUTA-001-A` remains `In Progress` until the residual GUI acceptance is delivered.

## Risks and assumptions

- GPUI was intentionally not pulled into this foundation checkpoint; the CLI validates the domain/media contract without prematurely defining final UI behavior.
- `ffprobe` is the initial deterministic probe backend; `MediaProbe` keeps the boundary replaceable.
- Imported assets reference originals rather than copying or taking ownership of them. Project-local derivatives/cache remain separate and reconstructable.

## Closure

Result: **partial** for `LAPUTA-001-A`; **complete** for the requested initial foundation checkpoint.
