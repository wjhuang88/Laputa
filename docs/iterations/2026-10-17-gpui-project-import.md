# Iteration — GPUI Project Import Closure

Status: partial  
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

## Delivered

- Added a separate, feature-gated `laputa-desktop` binary under `laputa-app`; the existing CLI is unchanged and remains usable.
- Pinned `gpui` / `gpui_platform` to the same Zed revision used by the currently pinned Talos desktop stack.
- Added a testable application presentation boundary (`DesktopState`) that does not depend on a graphical renderer.
- Implemented the Simplified-Chinese flat-Nordic launcher and import workspace with a visually dominant Stage, secondary Asset Browser and contextual Laputa panel.
- Implemented native project-folder selection for create/open and native multi-file selection for importing media.
- Moved project creation/opening and media probe/import work to GPUI background execution so synchronous filesystem/`ffprobe` work does not run on the UI thread.
- Reused `ProjectStore::import_media` and `FfprobeMediaProbe`; the GUI owns no duplicate import/provenance rules.
- Implemented Asset Browser projection from persisted `ProjectManifest` data with `全部 / 照片 / 视频` filters plus type/name/duration/dimension presentation.
- Opening an existing project restores and renders its persisted inventory without re-importing original media.
- Recoverable import/probe errors preserve the current visible project inventory and return desktop operation state to idle.
- Added focused tests for reopened inventory projection/filtering, recoverable failure state and overlapping-operation prevention.
- Added Linux CI native dependencies needed to compile the pinned GPUI renderer path while retaining the normal repository gates.

## Success criteria status

- [x] `laputa-desktop` builds behind an explicit `desktop-ui` feature and leaves existing CLI behavior available.
- [x] A new/empty project renders the import-oriented Stage with a primary `添加素材` action.
- [x] File selection supports multiple media files and routes them through the existing non-destructive importer.
- [x] Successful import state immediately projects the updated `ProjectManifest` into Asset Browser cards.
- [x] Opening an existing project projects its persisted Asset inventory without re-importing originals.
- [x] Import/probe errors are explicit and recoverable in the presentation state.
- [x] No model/provider selector, setup wizard, multi-track timeline, first-cut behavior or unrelated V0.1 feature was introduced.
- [ ] Real-desktop manual import/reopen smoke recorded.

## Validation evidence

Final clean branch CI: GitHub Actions run `39010083863`, job `117330603266`, Rust 1.95, **success**.

It ran with `contents: read` and no CI source mutation:

```sh
cargo fmt --all -- --check
cargo check --workspace --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked
cargo test -p laputa-app --features desktop-ui --bin laputa-desktop --locked
scripts/validate_project_governance.sh .
```

The full-feature validation includes the pinned GPUI desktop binary and the new `DesktopState` tests. An earlier compiler-driven run (`39009748674`) also completed successfully while materializing the final scroll-state fix; it is not used as the final clean evidence because that temporary branch-bootstrap step was removed before closure.

The existing `laputa-media` non-destructive import tests remain part of the workspace test gate and continue to protect original-byte immutability, rollback semantics and reopen behavior.

## Manual/visual residual

The current execution environment cannot provide an interactive user desktop with a real native file-picker session. The following required Story A evidence has therefore **not** been fabricated or replaced by headless CI:

1. launch `laputa-desktop` on a real graphical desktop;
2. create/open a project;
3. select a mixed set of supported photos/videos;
4. verify imported items appear in Asset Browser;
5. close the app;
6. relaunch and open the same project;
7. verify the same inventory is visible without re-importing originals.

Until this row is recorded, `LAPUTA-001-A` remains **In Progress** and this iteration remains **partial**.

## Risks and assumptions resolved

- GPUI file picker is sufficient for Story A because acceptance says picker **and/or** drag-and-drop. Drag-and-drop remains intentionally unimplemented here.
- Thumbnail generation is not required by Story A; the first Asset Browser uses designed media cards with type/name/technical metadata and does not expand the media pipeline.
- `ffprobe` remains the probe backend. Missing FFmpeg is surfaced as an import error rather than invented success.
- GPUI remains isolated to `laputa-app`; domain/media crates do not acquire a GUI dependency.

## Closure

Result: **partial**.

Implementation and automated validation are complete for this bounded iteration. The sole required residual is the real-desktop manual import/reopen smoke listed above.
