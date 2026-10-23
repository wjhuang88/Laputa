# Iteration — Film Workspace Visual Baseline

Status: in progress  
Date: 2026-10-23  
Selected Story: `LAPUTA-001-D` (visual checkpoint only; Story remains incomplete)  
Issue: #5

## Goal

Establish one production GPUI screen that is visually close enough to the approved Simplified-Chinese `docs/design/assets/v0.1/03-workspace.jpg` mockup to become the implementation baseline for subsequent Laputa desktop work.

The first target is the **First-cut Film Workspace** only. This iteration intentionally prioritizes composition, proportions, spacing, typography hierarchy, flat Nordic color blocks, Stage dominance, low-density side panels and the Scene Strip visual language before wiring the rest of Story D behavior.

## Change-control context

The prior D direction was ready to proceed function-first (`preview + scene strip + selection context`). The product direction is now explicitly changed: before continuing behavioral implementation, create one high-fidelity page to lock the visual tone.

This does not contradict ADR-0003. The interaction model remains asset browser + dominant central preview + lightweight scene strip + contextual Laputa panel. The sequencing changes: **visual calibration first, behavioral completion second**.

Foundation-only exception: this checkpoint is allowed to stop before full Story D acceptance because the approved mockup is itself an implementation dependency for consistent future GPUI work. The Story must remain open after this iteration unless its behavioral acceptance is independently satisfied.

## Published baseline

Primary reference:

- `docs/design/assets/v0.1/03-workspace.jpg`
- `docs/design/UI-UX-VISION-v0.1.md`
- ADR-0003

Expected user-visible result:

- warm off-white edge-to-edge app canvas;
- compact top bar with project title, undo/redo and export affordance;
- narrow `片段素材` support rail;
- substantially dominant central Stage / final preview surface;
- narrow contextual `Laputa` rail with restrained actions;
- shallow storyboard-like Scene Strip;
- Simplified Chinese copy and consistent visual tokens;
- noticeably more negative space and fewer nested cards than the current desktop workspace.

## Explicitly deferred from this checkpoint

- real video decoding/playback;
- real seek synchronization;
- timeline mutation;
- selection-scoped Talos commands;
- reorder/trim/delete/lock behavior;
- Story E/F/G/H work.

Static/demo presentation state may be used where necessary to reproduce the approved working-state composition, but it must be isolated so later real state can replace it without rewriting the visual system.

## Implementation order

1. Extract/normalize reusable workspace visual tokens and spacing constants from the approved design.
2. Refactor the GPUI workspace shell around the dominant Stage proportions.
3. Implement high-fidelity left Moments rail, Stage/player chrome, contextual Laputa rail and compact Scene Strip.
4. Add a deterministic demo/visual state so the page is reviewable even without running a live First Cut.
5. Keep existing project/import behavior intact.
6. Run fmt/check/clippy/workspace tests/desktop-ui/governance CI.
7. Record screenshots or user review outcome before treating this visual baseline as accepted.

## Current implementation checkpoint

- Branch: `feature/laputa-workspace-visual-baseline`.
- Added dedicated `laputa-workspace-visual` GPUI binary so visual calibration is isolated from the existing project/import desktop flow.
- The visual binary already encodes the approved hierarchy: 208 px Moments rail, dominant center Stage, 266 px Laputa rail and 112 px Scene Strip at the baseline desktop composition.
- Minimal demo interactions are intentionally limited to play/pause appearance and selected scene projection; they exist only to make the visual state reviewable.
- Existing `laputa-desktop` behavior remains unchanged in this checkpoint.

## Validation plan

Automated:

- `cargo fmt --all -- --check`
- `cargo check --workspace --locked`
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
- `cargo test --workspace --locked`
- `cargo test -p laputa-app --features desktop-ui --bin laputa-desktop --locked`
- `scripts/validate_project_governance.sh .`

Visual/manual:

- compare the running Film Workspace against `03-workspace.jpg` at approximately the approved 1586×992 / 16:10 composition;
- Stage must clearly dominate side rails;
- bottom strip must remain shallow/storyboard-like;
- no provider/model/workflow/NLE chrome may appear;
- user review is the acceptance gate for the visual direction.

## Risks / assumptions

- GPUI text metrics/platform fonts will prevent literal pixel equality; fidelity is judged on composition, hierarchy, density and visual language rather than raster-identical glyphs.
- The mockup contains media imagery; this iteration should not make external media availability a hard runtime dependency. Flat placeholders or repository-local design assets may support visual calibration where appropriate.
- Existing launcher/import functionality must not regress while the workspace view is refactored.
