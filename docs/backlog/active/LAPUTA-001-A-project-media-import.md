# LAPUTA-001-A — User can create a project and safely import mixed media

Parent: LAPUTA-001  
Type: Product Story  
Status: In Progress  
Dependencies: None  
Decision links: `docs/decisions/ADR-0003-agent-first-consumer-interaction.md`  
Design reference: `docs/design/UI-UX-VISION-v0.1.md`

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
- Thin developer CLI for `create`, `import`, and `open` remains available for deterministic foundation debugging.
- Automated evidence covers original-byte immutability, reopen, invalid-batch rollback, duplicate rejection and probe failure rollback.

Implemented desktop path in `2026-10-17-gpui-project-import.md`:

- Feature-gated `laputa-desktop` GPUI binary, separate from the existing CLI.
- GPUI launcher for creating a project directory or opening an existing Laputa project.
- System multi-file picker for importing local media; Story A intentionally satisfies the picker **and/or** drag-and-drop requirement via picker only.
- Import work runs off the UI thread and reuses `ProjectStore::import_media` + `FfprobeMediaProbe` rather than duplicating media rules in the GUI.
- Low-density Asset Browser renders persisted imported items with `全部 / 照片 / 视频` filters, media kind, filename and available duration/dimension metadata.
- Opening a persisted project projects its existing `ProjectManifest` inventory directly into the Asset Browser without re-importing originals.
- Import/probe failures return the desktop state to idle while preserving the already-visible project inventory and surfacing an explicit recoverable error.
- Presentation-state tests cover reopened inventory projection/filtering, recoverable failure state and prevention of overlapping desktop operations.
- GPUI dependencies are pinned to the same Zed/GPUI revision used by the currently pinned Talos desktop stack; GPUI remains isolated to the app presentation boundary.

Remaining before this Story can be Complete:

- Manual desktop import + close/relaunch + reopen smoke evidence on a real graphical desktop.

The Story therefore remains **In Progress** even after automated desktop build/test validation. A headless build or CI compile is not substituted for the required manual GUI evidence.

## GUI implementation guidance

Consumer UI work must read `docs/design/UI-UX-VISION-v0.1.md` before implementation.

- Use the **Empty project / media import** mockup (`docs/design/assets/v0.1/02-import.jpg`) as the primary visual reference for create/import behavior.
- Use the **First-cut workspace** mockup (`docs/design/assets/v0.1/03-workspace.jpg`) for the persistent workspace hierarchy into which the Asset Browser evolves.
- Preserve the approved low-density, flat Nordic visual direction and keep the central Stage visually dominant.
- Mockups are hierarchy/interaction references, not pixel-perfect acceptance criteria; this Story's acceptance and `PRODUCT-REQUIREMENTS-v0.1.md` remain behavior owners.
- Do not add setup wizards, model/provider selection, professional timeline controls or other visible configuration to satisfy this Story.

## Validation evidence

Foundation validation: GitHub Actions run `38860507105` passed on Rust 1.95 with workspace format/check/clippy/test/governance gates.

`laputa-media` has four focused tests for safe import/reopen and rollback semantics:

- `import_preserves_original_bytes_and_reopens_inventory`
- `invalid_batch_does_not_partially_mutate_manifest`
- `duplicate_import_is_rejected_without_manifest_change`
- `probe_failure_does_not_mutate_manifest_or_original`

The GPUI closure iteration records the desktop-specific build/test run after the final clean CI configuration is validated.

## State owners to synchronize

- `docs/backlog/PRODUCT-BACKLOG.md`
- parent LAPUTA-001 summary
- active iteration record
- `docs/design/UI-UX-VISION-v0.1.md` when approved GUI direction changes
- user-facing README if observable startup/import behavior changes
