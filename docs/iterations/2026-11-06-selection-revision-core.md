# Iteration — Selection Revision Core (F1)

Status: partial — F1 technical implementation delivered to PR #31; Story F remains open  
Date: 2026-11-06  
Selected Story: `LAPUTA-001-F` / Issue #7 (F1 only)  
Published planning baseline: `main` `948f43abf75615f96332ac386b8e3cd4717d5063`

## Goal

Deliver the **smallest deterministic, persisted edit core** shared by future Talos revision Tools and the product Scene Strip, without inventing a second timeline or blocking on real-desktop validation. This is **not Story F closure**.

## Dependency/readiness decision

- Story D / #5 has merged product Film Workspace, Scene Strip, Selection and successful automated product smoke. Its single outstanding real-desktop human gate is **MA-D-01** (see `docs/acceptance/MANUAL-ACCEPTANCE.md`).
- F1 reads only the merged canonical `FilmState`, `FilmStore`, and established `WorkspaceState::command_context` semantics. It does **not** claim D's human gate is passed.
- **Ready for this bounded technical slice only**. Complete Story F remains blocked on real product GUI/Talos integration and independent manual MA-F-01…04 acceptance. This exception avoids silently treating unfinished D as accepted.
- Per iteration SOP, this technical slice is a foundation exception: tests can confirm persisted edit behavior even before the GUI exposes it; F1 alone is **partial** user value.

## Fixed F1 scope / success criteria

1. Explicit scope overrides Selection; selected Scene is otherwise the default; no Selection selects whole project. Unknown/stale Scene IDs fail rather than silently broadening scope.
2. Reorder, trim, delete and lock/unlock mutate a **validated copy** of canonical `FilmState`; invalid scene ID, range, empty final timeline or locked-target edit fails without overwriting the last committed FilmState.
3. Lock persists in schema-v1 `film.json` using a backward-compatible default for old files. Automated revision may not mutate a locked Scene, silently unlock it, or reorder it to a different index.
4. Reorder is expressible without Talos; removing the last Scene from a story beat keeps state valid; actual source assets, originals and Asset IDs remain untouched.
5. A project-local persisted edit entrypoint can reopen FilmState, apply edits, validate against current ProjectManifest, and save only on success. Unit + on-disk fixture tests prove storage/reopen/rollback and source-byte preservation.

## Explicit exclusions / future handoff

- **F2**: `timeline.edit` Talos Tool + structured NL scope routing, explicit override semantics, and repair/lock enforcement at Tool boundary.
- **F3**: GPUI Scene Strip direct drag/reorder, trim/delete/lock affordances; preview invalidation, state binding and interaction automation.
- **F4**: generated artifact variant “Try another”, all real GUI manual MA-F-01…04 evidence, Story F acceptance/closure.
- No Story G critic or Story H final export; no vendor/model routing controls, multi-track UI or edits to source media.
- No schema-version bump for a backward-compatible optional boolean; see compatibility tests. If future metadata demands a schema change, track explicit migration.

## Validation plan

- Rust deterministic tests in `laputa-timeline`: scope precedence, stale scope, valid direct edit, lock enforcement, invalid source range, deletion invariant, serde-v1 compatibility, on-disk reopen & unchanged original bytes.
- Baseline exact-head push/PR CI: FFmpeg render fixture, fmt/check/clippy workspace/desktop tests and project governance. Record real run IDs/results here after execution.
- Real desktop **MA-D-01 / MA-F-01…04 remain UNRUN**. No human check is inferred from CI.

## Implemented F1 checkpoint (2026-11-06)

- The initial implementation and tests are in [PR #31](https://github.com/wjhuang88/Laputa/pull/31). Code head `7f47e644389b0004c0f41b53f2cf92442967fffd` has **push CI [42181097639](https://github.com/wjhuang88/Laputa/actions/runs/42181097639) completed / success**.
- Initial PR CI `42180780604` failed **only at `cargo fmt --all -- --check`**, after FFmpeg fixture success. Its exact Rustfmt hunks were applied, then branch rerun. No code behavior change was made for that formatting correction.
- The same code head's PR CI is [42181103778](https://github.com/wjhuang88/Laputa/actions/runs/42181103778); its outcome must be checked on GitHub before merge. Subsequent documentation-only updates require **fresh exact-head CI**; do not substitute these earlier run IDs for the new head.
- Source files: `crates/laputa-timeline/src/lib.rs`, `crates/laputa-timeline/src/revision.rs`, `crates/laputa-app/src/workspace_state.rs`. Verified in source: legacy Scene lock defaults false, candidate-first revision, scene ID/source preservation, atomic FilmStore save boundary, existing Selection-based scope.
- Added tests: explicit vs selected vs global scope, stale ID rejection, reordered/stable scene identity, source-bound trim, last-scene deletion rejection, orphaned-beat prune, user unlock and autonomous lock guard, legacy schema-v1 JSON, disk reopen/failed-revision rollback, unchanged original fixture bytes, and D Workspace Selection context.
- No vendor/model/tool routing changes, no use of paid credentials, no actual GUI drag/trim/lock, and no live/manual claim.

## Disposition and continuation

**Partial**: this technical F1 slice is implemented and submitted for validation; Story F itself is **not** Complete. Required later work is F2 Talos `timeline.edit` / explicit natural-language scope, F3 GPUI direct controls and preview invalidation, F4 verified variant retry/product end-to-end path. Do not close #7 or check any of its acceptance criteria yet. Real-desktop **MA-D-01** and future **MA-F-01…04** remain pending in the central manual acceptance ledger.

Final PR exact-head and post-merge main CI outcomes should be recorded on [Issue #7](https://github.com/wjhuang88/Laputa/issues/7) after they actually finish; they are not inferred from earlier runs.

