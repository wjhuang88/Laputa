# LAPUTA-001-J — Verify generation state persistence on macOS and Windows

Parent: LAPUTA-001  
Type: Technical verification / corrective follow-up Story  
Status: Complete — macOS/Windows local-runner persistence verified (post-merge CI / closeout gate)  
Issue: [#44](https://github.com/wjhuang88/Laputa/issues/44)  
Closed: 2026-11-27 (effective on closeout merge)  
Implementation: [PR #46](https://github.com/wjhuang88/Laputa/pull/46), main `a96e971b412579b7c5ba6a02a2605ee0a2c3f97a`  
Dependency: [LAPUTA-001-I](LAPUTA-001-I-generation-state-concurrency.md) (Linux-validated lost-update fix; Complete 2026-11-21)  
Iteration: [2026-11-21-generation-state-portability](../../iterations/2026-11-21-generation-state-portability.md)  
Required reads: `AGENTS.md`, `docs/sop/REQUIREMENT-INTAKE.md`, `docs/sop/TESTING.md`, `docs/sop/GIT-WORKFLOW.md`, `docs/reference/ARCHITECTURE.md`, archived LAPUTA-001-I

## Outcome

Verify that `GenerationStore`'s stable per-project OS lock and `NamedTempFile::persist` replacement semantics work on the supported macOS and Windows runner environments. Do not equate the previous Ubuntu CI success with a cross-platform guarantee.

## Scope

- Add an independently scoped CI workflow for generation persistence tests on macOS and Windows; it does not depend on provider credentials or a live remote model.
- Exercise the deterministic concurrent-thread, cross-process writers, stale-snapshot rejection and injected pre-replacement failure tests from the archived LAPUTA-001-I Story on both platforms.
- Confirm previously persisted jobs remain readable after reopen and failed writes do not delete the last committed state.
- Make a narrow platform-specific production correction *only if a verified failure requires it*, and update this Story plus its tests/evidence.

## Explicit exclusions

No changes to UI, provider selection, generation business semantics, Story E's historical acceptance, or the already-closed Ubuntu #40 defect. Avoid running full desktop/FFmpeg rendering suites on Windows/macOS when only persistence is being verified.

## Acceptance

- [x] GitHub Actions registered and ran macOS and Windows jobs (both completed with real test output on #44884955085 and #44884959371).
- [x] `cargo +1.95.0 test -p laputa-media --lib generation_store::tests:: --locked -- --nocapture` passed: macOS 9/9, Windows 9/9, including cross-process writers, state reopen and rollback.
- [x] Exact-head `050fe8699973fc9d26d3b82447c55abf29fee7c6`: macOS job [136788137609](https://github.com/wjhuang88/Laputa/actions/runs/44884959371/job/136788137609), Windows [136788137461](https://github.com/wjhuang88/Laputa/actions/runs/44884959371/job/136788137461), both success; associated push [#44884955085](https://github.com/wjhuang88/Laputa/actions/runs/44884955085) also success.
- [x] Technical change touched only workflow + governance documents. Import originals and production Rust code are unchanged; network-filesystem semantics are not claimed.
- [x] Exact-head CI and documented single-maintainer COMMENT reviews preceded merge (explicitly not independent approval).

## Risks / verification boundary

Early workflows failed with zero jobs because the inline YAML scalar contained a `::` test-filter fragment followed by whitespace and could not be parsed. Using `run: |` resolved the workflow registration; real macOS and Windows jobs later passed. OS lock behavior on external/network filesystems remains outside current runner coverage; it may require a separate environmental follow-up.

**Verified 2026-11-21:** actual pinned Rust 1.95 macOS and Windows tests passed at `050fe8699973fc9d26d3b82447c55abf29fee7c6`. [PR CI #44884959371](https://github.com/wjhuang88/Laputa/actions/runs/44884959371): both platform jobs success (9/9 tests each). [Push portability #44884955085](https://github.com/wjhuang88/Laputa/actions/runs/44884955085) and baseline Push/PR CI `44884955070`/`44884959301` all success. Initial inline YAML command produced zero-job invalid workflow; fixed by `run: |` block. **The Story remains In Progress only until synchronized main-branch CI and governance closeout**; no network-filesystem or paid-provider evidence is claimed.

## Final-head exact evidence and closeout scope

- Post-sync PR head: `273e3718fbe633eae02c44663c959392b23201a3`.
- [PR portability CI #46025102287](https://github.com/wjhuang88/Laputa/actions/runs/46025102287) SUCCESS: macOS job `140594351454` **9/9**, Windows job `140594350932` **9/9** on Rust 1.95. Logs include `cross_process_writers_preserve_every_job`, `stale_whole_snapshot_cannot_erase_a_newly_submitted_job` and `injected_replacement_failure_keeps_last_committed_state`.
- [PR baseline CI #46025102331](https://github.com/wjhuang88/Laputa/actions/runs/46025102331) and [push baseline CI #46025090671](https://github.com/wjhuang88/Laputa/actions/runs/46025090671): SUCCESS at same exact head.
- [Single-maintainer review #6090353746](https://github.com/wjhuang88/Laputa/pull/46#pullrequestreview-6090353746) on final PR head: COMMENT, **not independent approval**.
- Squash merge: [PR #46](https://github.com/wjhuang88/Laputa/pull/46), main `a96e971b412579b7c5ba6a02a2605ee0a2c3f97a`.
- Code-merge main post-merge gates: [portability CI #46026645868](https://github.com/wjhuang88/Laputa/actions/runs/46026645868) and [baseline CI #46026645696](https://github.com/wjhuang88/Laputa/actions/runs/46026645696), **must be SUCCESS before the documentation closeout PR is merged**.
- A separate governance closeout PR itself requires exact-head CI and maintainer self-review before completion.

**Result boundaries:** completed for GitHub-hosted macOS/Windows local filesystem behavior; no external/network filesystem, alternate Windows versions or live Runway provider test was executed or accepted. Historical Story I / #40 is not reopened. The unrelated intermittent decoder fixture defect #43 remains Open.
