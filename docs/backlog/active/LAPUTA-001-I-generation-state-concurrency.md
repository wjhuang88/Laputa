# LAPUTA-001-I — Concurrent-safe durable generation state updates

Parent: LAPUTA-001  
Type: Technical defect follow-up Story  
Status: Ready — triaged; implementation NOT STARTED  
Priority: P1 proposed (lost durable remote-job handle)  
Issue: [#40](https://github.com/wjhuang88/Laputa/issues/40) (reported by @mattabate)  
Historical dependency: LAPUTA-001-E / [#6](https://github.com/wjhuang88/Laputa/issues/6) (Complete/Closed 2026-11-04)  
Decision links: `docs/decisions/ADR-0002-functional-provider-abstraction.md`  
Required reads: `AGENTS.md`, `docs/sop/REQUIREMENT-INTAKE.md`, `docs/sop/CHANGE-CONTROL.md`, `docs/reference/ARCHITECTURE.md`, `docs/sop/TESTING.md`, archived Story E

## Outcome / change-control decision

Concurrent generation operations within one project must never erase already-persisted jobs, remote provider handles or published artifacts. Track this as a **new corrective child Story** of LAPUTA-001; preserve Story E's historical 2026-11-04 acceptance and keep #6 closed. This triage does not start implementation.

## Evidence status (2026-11-19)

**Confirmed by static code inspection of main `840c0141f6588a9353fe294edb637f9b01609260`:**

- `resume_video_generation` retains one loaded `GenerationState` across provider polling.
- `transition_job` persists the entire in-memory state, and `submit_video_generation` does a load/append/full-save. `finalize_video_artifact` also writes a whole snapshot; cancellation and failure flow through snapshot-based transitions.
- `GenerationStore::save` has no per-project read/modify/write synchronization and names its temp state file only by PID.
- A stale snapshot can overwrite a concurrently submitted job. Reloading inside only `transition_job` does not make read-modify-save atomic.

**Not yet verified:** deterministic runtime reproduction, cross-process/platform behavior, CI/test results, or any actual fixed code. The observed defect is supported by the code path, not a completed runtime test.

## Scope

- Provide a per-project atomic load/mutate/validate/save boundary across independent orchestrator instances and processes.
- Migrate submission, transition, failure/cancellation and artifact publication so unrelated job and artifact records are preserved and stale terminal states cannot be silently rolled back.
- Ensure distinct temp files per write and safe replacement/failure behavior on supported platforms.
- Do not hold state locks while awaiting provider submit/poll/cancel or long media materialization.
- Preserve provider-neutral contracts, original-media immutability, verified Artifact gates and durable provider provenance.

## Out of scope

No model/provider selection UI, vendor routing changes, broad storage redesign, new generation features or revision of Story E's past completion evidence. No external provider completion counts as verified Artifact publication.

## Acceptance / minimum evidence

- [ ] Deterministic interleaving: A polls while B submits; a subsequent A transition preserves both jobs and B's submitted provider handle after reopen.
- [ ] Concurrent submissions from independent orchestrators, plus multi-process writes on supported platforms, preserve all committed jobs without corrupting state.
- [ ] Cancellation, failure, completion and verified Artifact publication preserve unrelated jobs/artifacts; conflicting stale/terminal mutations fail safely.
- [ ] Simultaneous temp writes cannot collide; injected write/replacement failures leave previously committed state valid/readable.
- [ ] Tests cover original-media hashes unchanged, provider provenance/recovery/cancellation and verified Artifact eligibility, without a paid or live Runway dependency.
- [ ] Record results for `cargo fmt --all -- --check`, `cargo check --workspace --locked`, `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`, `cargo test --workspace --locked`, `scripts/validate_project_governance.sh .`, relevant platform tests, PR review, exact-head CI and post-merge verification.

## Delivery gate

Select this Ready Story into a bounded iteration before production changes. The governance PR records triage only. Keep [#40](https://github.com/wjhuang88/Laputa/issues/40) open until a separate implementation PR, regression/CI evidence and backlog/Epic updates are complete. No human or live-provider evidence is fabricated.
