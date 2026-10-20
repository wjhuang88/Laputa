# Iteration — GPUI Project Import Closure

Status: complete  
Date: 2026-10-17  
Closed: 2026-10-20  
Selected Story: `LAPUTA-001-A`  
Issue: #2

## Goal

Deliver the smallest user-reachable desktop slice that wires the existing non-destructive project/media foundation into GPUI: open/create a project shell, choose supported local photos/videos, import them without mutating originals, render the persisted Asset inventory in the approved low-density UI, and reopen the project with the same inventory restored.

## Published baseline

This iteration implemented only the remaining GUI acceptance of `LAPUTA-001-A`; it did not start media understanding, Talos First Cut, story/timeline generation, generative media, provider routing, playback, scene editing, or export.

Expected user-visible path:

`launch desktop -> create/open project -> choose media -> import -> see Asset Browser inventory -> close/relaunch/open -> same inventory rendered`

The implementation follows `docs/design/UI-UX-VISION-v0.1.md`; the Empty Project / Import mockup is the primary reference and the First-cut Workspace mockup defines the persistent workspace hierarchy.

## Delivered

- Separate feature-gated `laputa-desktop` binary; existing CLI remains usable.
- `gpui` / `gpui_platform` pinned to the same Zed revision used by the pinned Talos desktop stack.
- Renderer-independent `DesktopState` presentation boundary.
- Simplified-Chinese flat-Nordic launcher/import workspace with visually dominant Stage.
- Native project-folder selection and multi-file media selection.
- Project creation/opening and media probe/import work moved off the UI thread.
- Reused `ProjectStore::import_media` and `FfprobeMediaProbe`; GUI owns no duplicate import/provenance rules.
- Asset Browser projection from persisted `ProjectManifest` with `全部 / 照片 / 视频` filters and basic technical metadata.
- Existing projects restore and render inventory without re-importing originals.
- Recoverable import/probe errors preserve visible inventory and return operation state to idle.
- Focused tests cover reopened inventory projection/filtering, recoverable failure state and overlapping-operation prevention.
- Linux CI includes native packages required to compile the pinned GPUI renderer path.

## Success criteria status

- [x] `laputa-desktop` builds behind `desktop-ui` and leaves CLI behavior available.
- [x] Empty project renders the import-oriented Stage with primary `添加素材` action.
- [x] File selection supports multiple media files and routes through the non-destructive importer.
- [x] Successful import immediately projects the updated manifest into Asset Browser cards.
- [x] Reopen projects persisted inventory without re-importing originals.
- [x] Import/probe errors are explicit and recoverable.
- [x] No model/provider selector, setup wizard, multi-track timeline, first-cut behavior or unrelated feature was introduced.
- [x] Real-desktop manual import/reopen smoke recorded.

## Validation evidence

Automated evidence:

- Foundation CI `38860507105`: success.
- Final clean GPUI branch CI `39010083863`: success with read-only CI permissions and no source mutation.
- PR #12 CI `39010653178`: success.
- Post-merge `main` CI `39099379963`: success.

Validated gates include:

```sh
cargo fmt --all -- --check
cargo check --workspace --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked
cargo test -p laputa-app --features desktop-ui --bin laputa-desktop --locked
scripts/validate_project_governance.sh .
```

Manual evidence:

- The real graphical desktop path was exercised after merge, including the import/reopen behavior that could not be proven in headless CI.
- On 2026-10-20 the user explicitly confirmed the result was “符合预期”. This closes the previous residual `import -> close/relaunch -> reopen -> inventory visible` smoke row.

## Risks and assumptions resolved

- File picker is sufficient because Story A required picker **and/or** drag-and-drop; drag-and-drop remains optional future work.
- Thumbnail generation was not required by Story A; the first Asset Browser intentionally uses designed metadata cards.
- `ffprobe` remains the probe backend and missing FFmpeg remains a recoverable error.
- GPUI remains isolated to `laputa-app`; domain/media crates do not acquire GUI dependencies.

## Closure

Result: **complete**.

`LAPUTA-001-A` is dependency-closed and Story B may proceed.
