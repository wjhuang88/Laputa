# Iteration — Project and Media Foundation

Status: planned  
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

## Expected changes

- `laputa-domain`: Project/Asset identifiers, media kind, probe metadata, project manifest.
- `laputa-media`: filesystem project store, safe importer, media probe implementation/boundary, derivative paths.
- `laputa-app`: minimal command-line entrypoint for create/import/open until the GPUI surface is introduced by a later UI slice.
- tests: persistence/reopen and original-byte immutability fixtures.
- docs: Story/Epic/backlog/README updates only where observable behavior/status changes.

## Validation plan

Required baseline:

```sh
cargo fmt --all -- --check
cargo check --workspace --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked
scripts/validate_project_governance.sh .
```

Story-specific evidence:

- import fixture hashes match before/after import;
- derivative/project state is stored outside source media paths;
- project reopen restores imported inventory;
- unsupported/non-file inputs fail explicitly without partial manifest mutation.

## Risks and assumptions

- GPUI is intentionally not pulled into this Story; a minimal CLI makes the foundation runnable and testable without prematurely defining the final UI surface.
- `ffprobe` is treated as an optional deterministic probe backend. The importer must still preserve originals and report explicit probe failure when probing is unavailable or unsupported.
- Imported assets reference originals rather than copying or taking ownership of them. Project-local derivatives/cache remain separate and reconstructable.

## Closure rule

This iteration can close `complete` only when the runnable import/reopen path and required automated evidence pass. If the environment cannot execute required Rust checks, closure remains `partial` even if code is committed.
