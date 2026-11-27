# Iterations

Laputa iterations are auditable execution batches, not reusable Sprint names. Select Ready child Stories, not bare Epics.

Use `docs/sop/START-ITERATION.md` to create a new iteration record and `docs/sop/ITERATION-WORKFLOW.md` to execute/close it. Each product iteration should yield a runnable/testable result unless an explicit foundation exception is recorded.

Current/latest records:

- `2026-11-21-generation-state-portability.md` — LAPUTA-001-J / #44 macOS/Windows generation-store suite 9/9 on each OS, PR #46 merged as `a96e971`; post-merge main CI #46026645868/#46026645696 required before closeout is effective.

- `2026-11-21-scene-inpoint-thumbnails.md` — Story D / #5 source in-point video thumbnails merged in PR #47, exact-head CI success, post-merge CI tracked separately; human MA-D-VIS-01 remains FAIL and MA-D-01 PENDING.

- `2026-11-19-generation-state-concurrency-repair.md` — LAPUTA-001-I / #40, PR #42 merged; Ubuntu-backed transactional generation state fix accepted on closeout after main CI #44863056584, with macOS/Windows verification tracked under #44 and CI fixture flake #43.

- `2026-11-16-workspace-visual-rework.md` — PR #39 V1 layout and asset thumbnails merged and post-merge CI success; PR #47 cut-inpoint follow-up merged. Real macOS photographic fidelity MA-D-VIS-01 FAIL and MA-D-01 PENDING remain the open gates.

- `2026-11-14-f3b1-scene-drag-reorder.md` — native GPUI Scene Strip drag/release reorder slice in implementation; pointer acceptance, exact-head CI, F3b language/trim and F4 remain pending.

- `2026-11-14-f3a-direct-scene-editing.md` — F3a product GUI direct edit buttons, save/reopen and preview refresh merged by PR #34 (human preflight UNRUN); F3b drag/language input and MA-F-01…04 remain pending.

- `2026-11-07-talos-selection-revision.md` — F2 Talos timeline.get/timeline.edit technical checkpoint merged by PR #32; Story F still in progress pending F3/F4 and manual acceptance, MA-D-01 pending.

- `2026-11-06-selection-revision-core.md` — F1 deterministic revision core merged by PR #31, main CI #42183222768 success; Story D human gate remains open and Story F is not complete.

- `2026-10-31-generative-media-foundation.md` — complete; Story E / Issue #6 archived and closed (E1–E5b), credential-gated Runway live smoke explicitly unrun and optional.
- `2026-10-23-workspace-visual-baseline.md` — historically closed; current visual acceptance superseded by 2026-11-16 FAIL/rework; Story D also retains MA-D-01.

Human validation handoff: [manual acceptance ledger](../acceptance/MANUAL-ACCEPTANCE.md). **MA-D-01 is pending real-desktop confirmation**; do not infer acceptance from automated CI or mark future/optional tests completed.

Earlier iteration records are retained in this directory as execution history.
