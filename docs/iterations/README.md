# Iterations

Laputa iterations are auditable execution batches, not reusable Sprint names. Select Ready child Stories, not bare Epics.

Use `docs/sop/START-ITERATION.md` to create a new iteration record and `docs/sop/ITERATION-WORKFLOW.md` to execute/close it. Each product iteration should yield a runnable/testable result unless an explicit foundation exception is recorded.

Current/latest records:

- `2026-11-07-talos-selection-revision.md` — F2 Talos timeline.get/timeline.edit technical checkpoint merged by PR #32; Story F still in progress pending F3/F4 and manual acceptance, MA-D-01 pending.

- `2026-11-06-selection-revision-core.md` — F1 deterministic revision core merged by PR #31, main CI #42183222768 success; Story D human gate remains open and Story F is not complete.

- `2026-10-31-generative-media-foundation.md` — complete; Story E / Issue #6 archived and closed (E1–E5b), credential-gated Runway live smoke explicitly unrun and optional.
- `2026-10-23-workspace-visual-baseline.md` — complete for its visual-baseline goal; Story D retains a separate final real-desktop acceptance residual in Issue #5.

Human validation handoff: [manual acceptance ledger](../acceptance/MANUAL-ACCEPTANCE.md). **MA-D-01 is pending real-desktop confirmation**; do not infer acceptance from automated CI or mark future/optional tests completed.

Earlier iteration records are retained in this directory as execution history.
