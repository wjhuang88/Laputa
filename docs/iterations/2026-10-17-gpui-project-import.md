# Iteration — GPUI Project Import Closure

Status: planned  
Date: 2026-10-17  
Selected Story: `LAPUTA-001-A`  
Issue: #2

## Goal

Deliver the smallest user-reachable desktop slice that wires the existing non-destructive project/media foundation into GPUI: open/create a project shell, choose supported local photos/videos, import them without mutating originals, render the persisted Asset inventory in the approved low-density UI, and reopen the project with the same inventory restored.

## Published baseline

This iteration implements only the remaining GUI acceptance of `LAPUTA-001-A`.

It does **not** start media understanding, Talos first-cut behavior, story/timeline generation, generative media, provider routing, playback, scene editing, or export.

Expected user-visible path at closure:

`launch desktop -> create/open project -> choose media -> import -> see Asset Browser inventory -> close/relaunch/open -> same inventory rendered`

The implementation follows `docs/design/UI-UX-VISION-v0.1.md`; the Empty Project / Import mockup is the primary reference and the First-cut Workspace mockup defines the persistent workspace hierarchy. The mockups are hierarchy/interaction references, not pixel-perfect acceptance contracts.

## Dependency order

1. Add a feature-gated GPUI desktop binary without breaking the existing developer CLI.
2. Introduce a small presentation/state boundary that can be tested without an interactive display.
3. Render the approved flat Nordic project/import shell and Asset Browser from `ProjectManifest` state.
4. Wire GPUI multi-file picker to the existing `ProjectStore::import_media` + `FfprobeMediaProbe` path off the UI thread.
5. Support create/open project startup arguments so reopen can reconstruct the same inventory deterministically.
6. Add focused state/presentation tests plus Linux feature-build CI coverage.
7. Run workspace/governance validation and record actual evidence.

## Success criteria

- `laputa-desktop` builds behind an explicit `desktop-ui` feature and leaves existing CLI behavior available.
- A new/empty project shows the approved import-oriented Stage with a primary `添加素材` action.
- File selection allows multiple media files and sends them through the existing non-destructive importer.
- Imported assets become visible in a left-side Asset Browser without requiring a restart.
- Opening an existing project renders its persisted Asset inventory from `ProjectManifest` without re-importing originals.
- Import/probe errors remain explicit and recoverable; the UI never reports success when `ProjectStore::import_media` failed.
- No model/provider selector, setup wizard, multi-track timeline, or unrelated V0.1 feature is introduced.

## Validation plan

Automated:

```sh
cargo fmt --all -- --check
cargo check --workspace --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked
scripts/validate_project_governance.sh .
```

Desktop-specific CI must additionally compile/check the GPUI feature on Linux with the native packages required by the pinned GPUI renderer stack.

Focused tests will verify presentation projection from persisted assets, empty/imported/reopen states, and recoverable import error state without requiring a graphical display.

Manual/visual:

- launch the GPUI binary on a real desktop;
- create/open a project;
- select mixed supported media;
- verify imported inventory appears;
- close/relaunch/open the same project and verify inventory remains visible.

If the current execution environment cannot provide an interactive desktop, record that exact smoke row as residual and keep Story closure `partial` rather than inventing evidence.

## Risks and assumptions

- GPUI file selection is sufficient for Story A because acceptance permits file picker **and/or** drag-and-drop; drag-and-drop is intentionally not required in this iteration.
- Asset Browser may initially use designed media cards with type/filename/technical metadata rather than generated thumbnails; thumbnail generation is not required by Story A acceptance.
- `ffprobe` remains the probe backend; a missing local FFmpeg installation must surface as a recoverable import error.
- GPUI remains isolated in the application presentation boundary; Laputa domain/media crates must not depend on it.

## Closure

Result: pending.
