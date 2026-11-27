# LAPUTA-001-J — Verify generation state persistence on macOS and Windows

Parent: LAPUTA-001  
Type: Technical verification / corrective follow-up Story  
Status: In Progress — macOS/Windows tests passed; merge and post-merge verification pending  
Issue: [#44](https://github.com/wjhuang88/Laputa/issues/44)  
Dependency: [LAPUTA-001-I](../archive/LAPUTA-001-I-generation-state-concurrency.md) (Linux-validated lost-update fix; Complete 2026-11-21)  
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
- [ ] Relevant documentation and scoped CI remain synchronized; no unsupported platform claims and no altered original user media.
- [ ] Exact-head CI and documented single-maintainer code review (not independent approval) precede merge.

## Risks / verification boundary

A preceding workflow attempt in PR #42 showed zero jobs and was removed. The cause remains unverified; this slice first attempts an explicit two-job workflow rather than inferring a platform failure. OS lock behavior on external/network filesystems remains outside current runner coverage; it may require a separate environmental follow-up.

**Verified 2026-11-21:** actual pinned Rust 1.95 macOS and Windows tests passed at `050fe8699973fc9d26d3b82447c55abf29fee7c6`. [PR CI #44884959371](https://github.com/wjhuang88/Laputa/actions/runs/44884959371): both platform jobs success (9/9 tests each). [Push portability #44884955085](https://github.com/wjhuang88/Laputa/actions/runs/44884955085) and baseline Push/PR CI `44884955070`/`44884959301` all success. Initial inline YAML command produced zero-job invalid workflow; fixed by `run: |` block. **The Story remains In Progress only until synchronized main-branch CI and governance closeout**; no network-filesystem or paid-provider evidence is claimed.
