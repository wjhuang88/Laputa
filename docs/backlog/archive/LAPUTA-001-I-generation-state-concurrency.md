# LAPUTA-001-I — Concurrent-safe durable generation state updates

Parent: LAPUTA-001  
Type: Technical defect follow-up Story  
Status: Complete for the Linux-validated lost-update correction, contingent on closeout gate  
Closed: 2026-11-21 (effective when the governance closeout PR is merged)  
Reported issue: [#40](https://github.com/wjhuang88/Laputa/issues/40) (thanks @mattabate)  
Implementation: [PR #42](https://github.com/wjhuang88/Laputa/pull/42), merged to main at `0dfe62d94da6f95f83f7caab3f1bd29cc31d6c65`  
Iteration: [2026-11-19-generation-state-concurrency-repair](../../iterations/2026-11-19-generation-state-concurrency-repair.md)  
Historical parent: Story E / [#6](https://github.com/wjhuang88/Laputa/issues/6), Complete/Closed 2026-11-04 (unchanged)

## Correction / scope

The originally reported bug let a long-running A poll retain an obsolete `GenerationState` and overwrite a newly persisted B job/provider handle when a full snapshot was saved. This post-closure Story addressed that lost-update correctness defect without altering Story E's historical acceptance.

- `GenerationStore::update` serializes a project-scoped current-state read/mutate/validate/persist operation using a stable OS-lock file. `save` also rejects stale snapshots that remove committed jobs/artifacts or illegally change job transitions.
- Submission, job status transitions, cancellation/failure and publication update the current committed state, not a long-lived snapshot. Pending provider operations and materialization never hold the state lock.
- Unique temporary files eliminate same-PID temp-name collisions; failure injection proves a failed pre-replacement operation does not erase the last committed generation state.
- Artifact lineage, verified publication and original-media hashes remain protected. No provider/model picker, new vendor integration or timeline mutation was introduced.

## Exact-head automated evidence

- **PR head:** `de49fbb78d68eed6facf0736f6ed9c40d3c24005`.
- [Push CI #44457205819](https://github.com/wjhuang88/Laputa/actions/runs/44457205819): success; `cargo fmt --all -- --check`, `cargo check --workspace --locked`, `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`, `cargo test --workspace --locked`, desktop test, FFmpeg fixture and governance validator.
- [PR CI #44457211375](https://github.com/wjhuang88/Laputa/actions/runs/44457211375): attempt 1 failed in unrelated `laputa-render/tests/preview_decode.rs` (`ETXTBSY`); attempt 2 **success** at unchanged exact head. The unrelated flake remains tracked by [#43](https://github.com/wjhuang88/Laputa/issues/43).
- [Single-maintainer COMMENT self-review #5969243846](https://github.com/wjhuang88/Laputa/pull/42#pullrequestreview-5969243846): recorded with patch and gate checks, **not** independent approval.
- Main after code merge: [CI #44863056584](https://github.com/wjhuang88/Laputa/actions/runs/44863056584) on `0dfe62d94da6f95f83f7caab3f1bd29cc31d6c65`. **Success of this run is a precondition for merging the documentation closeout PR**, not asserted before it is observed.

## Acceptance disposition (current Ubuntu CI target)

- [x] Deterministic A-polls/B-submits reproduction preserves both jobs and the later provider handle after reopening.
- [x] Independent orchestrator and concurrent in-process/cross-process writer tests preserve every committed job on Ubuntu.
- [x] Stale/terminal transitions fail without resurrecting cancellation or overwriting unrelated records; verified Artifact publication preserves B.
- [x] Temp files have distinct names; invalid writes and injected pre-install failure retain previous readable state.
- [x] Original asset digest and provider provenance are retained; verification uses Fake providers and FFmpeg fixtures, not paid Runway.
- [x] Exact-head push and PR CI completed successfully; single-maintainer self-review explicitly disclosed.
- [ ] macOS/Windows OS lock, atomic replacement and reopen semantics were **not exercised** in the available Ubuntu-only baseline CI; tracked as [#44](https://github.com/wjhuang88/Laputa/issues/44) and **not** claimed as accepted.
- [ ] Credential-gated live Runway test is intentionally unrun/optional, as in historical Story E. It is not fabricated as acceptance evidence.

## Explicit closeout / residual decision

The primary lost-update defect is accepted for the repository's currently exercised Ubuntu CI environment, with no evidence of failed #40 tests. Non-Linux portability validation is a separately owned follow-up [#44](https://github.com/wjhuang88/Laputa/issues/44); this does not silently certify macOS/Windows or erase the recorded platform risk. CI flake [#43](https://github.com/wjhuang88/Laputa/issues/43) is unrelated to generation state.

Do not mark this Story fully cross-platform verified. The closeout record and [#40](https://github.com/wjhuang88/Laputa/issues/40) should be finalized only after code-merge main CI #44863056584 and governance closeout CI are green; preserve issue and run URLs as evidence. Story E / #6 remains historically closed.

## 2026-11-27 portability verification addendum

Follow-up [LAPUTA-001-J](LAPUTA-001-J-generation-state-portability.md) / [Issue #44](https://github.com/wjhuang88/Laputa/issues/44) verified the existing generation state implementation on GitHub-hosted macOS and Windows runner local filesystems (9/9 tests on each OS at PR #46 synchronized head). This complements, rather than changes, the original 2026-11-21 Linux-only acceptance record. Network filesystems and live Runway remain outside the verified scope.
