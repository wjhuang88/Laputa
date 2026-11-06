# Iteration — Selection Revision Core (F1)

Status: planned — F1 technical slice; Story F remains open  
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

## Evidence / disposition

Pending implementation and CI. Publish verified results here on the same branch before merge. The Story, Epic, Product Backlog and #7 must not claim F1 completes all Story F acceptance.
