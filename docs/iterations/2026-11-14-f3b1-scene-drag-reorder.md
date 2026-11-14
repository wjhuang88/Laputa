# Iteration — F3b1 Native Scene Strip Drag Reorder

Status: in progress — implementation proposed; exact-head CI and real-desktop acceptance pending  
Date: 2026-11-14  
Selected: `LAPUTA-001-F` / Issue #7, **bounded F3b1 drag-only**  
Published baseline: `main` `37b105c5c5eeb182df6d358180608b484301f66c` (F3a code PR #34, checkpoint PR #35 already merged)

## Goal

Enable true GPUI mouse drag from a Scene Strip card onto another Scene card (**insert before**) or the terminal drop zone (**move to end**). Preserve the existing canonical F1 edit semantics, project persistence, Selection handling and FFmpeg preview epoch fencing in F3a; do not simulate pointer drag using reorder buttons or force an Agent turn.

## Dependency and bounded readiness

Story D's canonical Film Workspace/Selection product code and F1/F2/F3a are merged. MA-D-01 remains a real-desktop human acceptance blocker for Story D. This F3b1 implementation relies only on already merged technical contracts and does not claim Story D or complete Story F acceptance. Story F remains Open until F3b natural-language/trim, F4 and manual checks are resolved.

## Acceptance for this slice

1. A mouse gesture drags one scene card and releases it on a second scene card to insert the dragged scene before the drop target; release on the trailing strip target moves it to the end. An ordinary click still selects the scene.
2. Drag payload contains stable Scene ID rather than card index; edit resolves both IDs against **current persisted** FilmState. Stale, missing, locked, or invalid changes fail without partial disk/UI writes.
3. Accepted reorders use F1 `FilmState::revised` and `FilmStore::apply_revision`; preserve valid Selection, stop old playback and rerender preview with F3a's async epoch fence.
4. Deterministic tests cover drag-intent reorder before/end, reopen persistence, a retained Scene's rejection/rollback and source-media immutability.
5. The real GPUI desktop compiles and product smoke runs in CI. Actual pointer behavior is a separately tracked real-desktop manual check; never infer human PASS from automation.

## Exclusions

- No provider/model/workflow selector, Talos turn, external API call, paid credentials or new database/schema.
- No claim that F3b language submit or fine trim controls or F4 variant retry are implemented.
- No changes to Story D, G/H semantics or user-facing multi-track editing.

## Validation plan and evidence

- `cargo fmt --all -- --check`
- `cargo check --workspace --locked`
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
- `cargo test --workspace --locked`
- Product desktop FFmpeg / Xvfb smoke, governance validation and exact-head push/PR CI.
- Real desktop: drag Scene 02 before Scene 01; confirm new order/preview/reopen; drag to end to restore order; try locked source and confirm no change; compare source files. Record environment, tested SHA, screenshots/recording and PASS/FAIL/BLOCKED separately in manual ledger.

## Disposition

**Planned.** Work is limited to the drag slice. F3b natural-language submit and richer trimming remain pending, as does F4. MA-D-01 remains CURRENT/REQUIRED; full MA-F-01…04 stay pending and must not be pre-checked. Complete this record with actual CI/review results, not predictions.
