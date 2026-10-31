# Iteration — Film Workspace Visual Baseline

Status: complete  
Date: 2026-10-23  
Closed: 2026-10-31  
Selected Story: `LAPUTA-001-D` (visual checkpoint only; Story remains incomplete pending final real-desktop acceptance)  
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

## Delivered checkpoint

- Development branch: `feature/laputa-workspace-visual-baseline`.
- A development-only Cargo example `workspace_visual` hosted the deterministic calibration page; `laputa-desktop` remained the **only product binary**.
- The accepted composition uses the intended narrow support rails, dominant Stage and shallow Scene Strip.
- PR #17 merged the accepted visual baseline as `b8e0c3e355cc43d2b617e71fa311b3e4aa1a6085`.
- Subsequent Story D slices #18/#19/#20 and canonical product binding #21 preserved this visual direction while wiring real state, FFmpeg preview and playback.
- PR #21 later merged the canonical product workspace to `main` as `f6b95aadd766502599b8c76dbdc0df695852b989`; post-merge CI `40720360492` succeeded.

Run the visual calibration page with:

```sh
cargo run -p laputa-app --features desktop-ui --example workspace_visual
```

This example is development tooling, not a second user-facing application entrypoint.

## Validation evidence

Automated validation for the visual checkpoint and subsequent Story D stack covered:

- `cargo fmt --all -- --check`
- `cargo check --workspace --locked`
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
- `cargo test --workspace --locked`
- desktop-ui product target validation
- project governance validation

Visual review accepted the `03-workspace.jpg` direction. Later product-level Xvfb smoke in PR #21 also confirmed that the canonical `laputa-desktop` preserved the accepted workspace while showing real preview and Scene Strip state.

## Closure result

**complete** for this iteration's stated visual-baseline goal.

This closure does **not** close Story D. Issue #5 still retains one independent residual: real-desktop human interaction acceptance of the canonical product workspace. That residual is tracked by #5 and does not keep this historical visual-baseline iteration open.

## Risks / assumptions disposition

- GPUI text metrics/platform fonts did not require raster-identical glyphs; acceptance was based on composition, hierarchy, density and visual language as planned.
- External media was not made a visual-baseline runtime dependency.
- Launcher/import behavior remained intact through the later canonical product binding.
