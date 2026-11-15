# Iteration — F3b2 Bounded Fine Trim Controls

Status: planned — implementation and exact-head CI pending  
Date: 2026-11-15  
Selected: `LAPUTA-001-F` / Issue #7, **bounded F3b2 trim-only**  
Published baseline: `main` `310efecb87cdac70312907da5d99ffd55bff6344` (F3b1 drag implementation merged by PR #36, independent main CI #43732451595 under verification)

## Goal

Expose simple, non-destructive **500ms source-range adjustments** for the selected scene in the real GPUI Film Workspace. Images adjust visible duration; videos can shift start or end boundaries independently without altering original bytes. Keep F1 canonical validation/persistence, F3a preview epoch fencing, and user-facing simplicity. This is incremental fine trim, **not** a draggable range-handle or a professional editor.

## Readiness and dependencies

Story D's UI contracts and F1/F2/F3a/F3b1 code are merged. Story D MA-D-01 remains pending; this slice relies only on technical contracts and cannot serve as human acceptance of D or F.

## Success criteria

1. Selecting an image exposes duration `-0.5秒 / +0.5秒`; selecting a video exposes independent start `-0.5秒 / +0.5秒` and end `-0.5秒 / +0.5秒` adjustments. UX must label the boundary and not expose low-level FFmpeg/Provider controls.
2. Changes are bounded to the selected stable Scene ID and use `FilmState::revised(..., RevisionOrigin::User)` / `FilmStore::apply_revision`; locked, invalid, out-of-range and stale project revisions reject without changing saved FilmState or GUI projection.
3. Successful edits preserve Selection, reproject time/preview and rerender through F3a epoch invalidation, without touching imported originals.
4. Deterministic tests cover start/end/image duration adjustments, source bounds, locked rejection, save/reopen, original hashes and no partial edits.
5. Exact-head fmt/workspace check/Clippy/tests, desktop binary and FFmpeg product CI success. Real desktop human smoke remains separately marked UNRUN until actual evidence.

## Non-goals

- No generation, direct manipulation of uploaded originals, drag trim handles, multi-track UI or agent turn.
- No GPUI natural-language submit yet; F3b language input/provider configuration remain future work.
- No F4 variant retry, G/H behaviors, external paid API or provider/model selection.

## Validation

`cargo fmt --all -- --check`, `cargo check --workspace --locked`, `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`, `cargo test --workspace --locked`, real FFmpeg fixture, desktop product test and governance validation, exact-head push/PR/post-merge checks. Record actual CI status, source commit and review evidence only once confirmed.

## Manual test handoff

Use the canonical non-private two-scene fixture from `docs/acceptance/MANUAL-ACCEPTANCE.md`. Select the image and adjust duration; select video and shift each boundary 500ms in both directions. Confirm preview length/order and saved state after reopening; intentionally attempt invalid/locked edits and confirm no mutation. Record OS/date/commit and PASS/FAIL/BLOCKED. This manual pass is a future human gate; CI is not a substitute.

## Disposition

**Planned.** Close this record only with actual evidence; Story F / Issue #7 stays Open. F3b natural-language submit, possible draggable trim handles, F4 variants and MA-F-01…04 remain separate.
