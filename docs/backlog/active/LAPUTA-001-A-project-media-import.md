# LAPUTA-001-A — User can create a project and safely import mixed media

Parent: LAPUTA-001  
Type: Product Story  
Status: In Progress  
Dependencies: None  
Decision links: `docs/decisions/ADR-0003-agent-first-consumer-interaction.md`

As a person making a personal video,  
I want to create a project and add local photos/videos without worrying about damaging them,  
so that I can start creating while my originals remain safe.

## Scope

- Project identity/location and persisted minimal project metadata.
- Import by file picker and/or drag-and-drop on the desktop shell.
- Asset records referencing originals; basic probe metadata and project-local derivative/cache locations.
- Original immutability contract and hash-based test fixture evidence.
- Project reopen with imported asset inventory restored.

## Out of scope

- AI analysis, generation, first cut, cloud sync, account system, mobile import.

## Acceptance

- Given a new Laputa project  
  When the user imports a fixture containing supported photos and videos  
  Then the asset browser shows imported items  
  And original files remain byte-for-byte unchanged.
- Given an imported project  
  When Laputa creates metadata/proxies/thumbnails  
  Then derivatives are stored separately from originals.
- Given the application is closed after import  
  When the same project is reopened  
  Then the imported asset inventory can be reconstructed/restored without re-importing originals.

## Minimum validation

- Automated fixture test records original hashes before/after import and derivative generation.
- Rust workspace checks/tests for affected crates.
- Manual desktop smoke check for import and reopen when GUI path exists.

## Current implementation checkpoint

Completed foundation:

- Persisted versioned `ProjectManifest` and immutable original `Asset` references.
- Project-local `.laputa/` state, cache and derivatives layout.
- Non-destructive batch import with SHA-256 provenance and duplicate/type validation.
- `MediaProbe` boundary with an `ffprobe` implementation.
- Manifest commit only after all input validation/probing and derivative-directory preparation succeeds.
- Reopen restores the imported asset inventory.
- Thin developer CLI for `create`, `import`, and `open` so the foundation is runnable before GPUI is introduced.
- Automated evidence covers original-byte immutability, reopen, invalid-batch rollback, duplicate rejection and probe failure rollback.

Remaining before this Story can be Complete:

- GPUI desktop file picker and/or drag-and-drop import path.
- Asset Browser rendering of imported items.
- Manual desktop import/reopen smoke evidence through the GUI path.

## Validation evidence

GitHub Actions `Foundation bootstrap validation` run `38860507105` passed on Rust 1.95 with:

- `cargo fmt --all -- --check`
- `cargo check --workspace --locked`
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
- `cargo test --workspace --locked`
- `scripts/validate_project_governance.sh .`

`laputa-media` currently has four passing focused tests for safe import/reopen and rollback semantics.

## State owners to synchronize

- `docs/backlog/PRODUCT-BACKLOG.md`
- parent LAPUTA-001 summary
- active iteration record
- user-facing README if observable startup/import behavior changes
