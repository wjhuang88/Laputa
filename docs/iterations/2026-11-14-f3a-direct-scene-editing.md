# Iteration — F3a Direct Scene Editing and Preview Refresh

Status: planned — bounded F3a product slice; Story F remains In Progress  
Date: 2026-11-14  
Selected: `LAPUTA-001-F` / Issue #7 (F3a)  
Baseline: `main` `a886020a11be98729bb35e5c296f7ef6dd9f5edc` (F1 PR #31 + F2 PR #32 + docs #33 merged; main CI #42344231989 success)

## Goal

Expose the merged F1 canonical timeline revision core in the **real GPUI `laputa-desktop` Film Workspace**, giving users immediate, source-preserving scene actions and correctly invalidating/rebuilding preview after each successful edit. No provider credential or Talos turn is required for direct manipulation.

## Readiness/dependency

Story D Film Workspace/Selection/Preview production code and automated Xvfb smoke are merged; its real desktop MA-D-01 gate remains pending. F3a uses those existing merged contracts only and is a bounded independently testable product slice; it does not claim D or Story F has passed real-desktop user acceptance.

## Fixed acceptance for F3a

1. A selected scene exposes discrete, direct actions to **move earlier/later**, **shorten by a clearly labeled 20%**, **remove**, **lock/unlock**. Moving must preserve Scene ID and media source; operations on a locked target cannot silently mutate it. Lock toggle is explicit user action, not an Agent turn.
2. All operations reuse `FilmState::revised` / `FilmStore::apply_revision`; on error, original `film.json` and GUI projection remain stable. Deleting the last Scene is rejected; locked scenes are protected.
3. After successful persistence, rebuild the `WorkspaceState`, retain valid Selection (or clear deleted selection), pause playback, clamp seek, invalidate existing cached playback/frame and re-render preview using the new timeline.
4. Async preview render results started before the revision must be ignored; a stale render may not overwrite the new preview. No source original is changed or deleted.
5. Headless Rust tests cover scene move/shorten/delete/lock, reopened persisted state, Scene Strip order and preview plan, failure rollback, and unaffected original bytes. Normal desktop binary CI compile/smoke proves GPUI UI wiring; real-desktop human MA-F verification remains pending.

## Explicit exclusions / follow-on

- F3b: true pointer drag-and-drop reorder, a real text editing / submit control wired to Talos `RevisionRunner` with configured provider, advanced trim range interaction. This slice provides discrete arrow moves, not drag acceptance.
- F4: generated artifact variant “Try another”; no video color grading claims.
- No multi-track editor, external paid provider, Talos core change, source media mutation, or structural schema redesign.

## Validation

`cargo fmt --all -- --check`; `cargo check --workspace --locked`; `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`; `cargo test --workspace --locked`; product desktop FFmpeg/Xvfb smoke; repository governance. Run evidence, exact heads, PR review and post-merge main CI must be recorded after actual results.

**Manual**: MA-D-01 remains CURRENT/REQUIRED and MA-F-01…04 FUTURE/NOT READY. Do not mark PASS solely from CI.

## Checkpoints

Pending implementation/CI. Preserve this baseline and record observed results without silently expanding the scope.
