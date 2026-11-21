# LAPUTA-001-J — Verify generation state persistence on macOS and Windows

Parent: LAPUTA-001  
Type: Technical verification / corrective follow-up Story  
Status: In Progress — portable CI validation pending  
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

- [ ] GitHub Actions successfully registers and starts macOS and Windows jobs (not merely zero-job workflow attempts).
- [ ] `cargo +1.95.0 test -p laputa-media --lib generation_store::tests:: --locked -- --nocapture` passes on both platforms, including cross-process writers and state rollback protection.
- [ ] Evidence includes exact Git SHA, macOS and Windows Job IDs, failing/retried attempts if any, and a clear closeout decision.
- [ ] Relevant documentation and scoped CI remain synchronized; no unsupported platform claims and no altered original user media.
- [ ] Exact-head CI and documented single-maintainer code review (not independent approval) precede merge.

## Risks / verification boundary

A preceding workflow attempt in PR #42 showed zero jobs and was removed. The cause remains unverified; this slice first attempts an explicit two-job workflow rather than inferring a platform failure. OS lock behavior on external/network filesystems remains outside current runner coverage; it may require a separate environmental follow-up.

**Evidence status:** UNRUN as of branch creation. Issue #44 remains Open until actual platform evidence is captured.
