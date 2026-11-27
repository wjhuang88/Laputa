# Iteration — Generation State Portability Verification

Status: Complete for GitHub-hosted macOS/Windows local-filesystem checks on governance closeout (network FS unverified)  
Date: 2026-11-21  
Selected Story: LAPUTA-001-J — Ready at selection, now In Progress  
Issue: [#44](https://github.com/wjhuang88/Laputa/issues/44)  
Published baseline: main `adabf39fc75da512f131d002fdaa7f978a3ff2b2`  
Prerequisite: LAPUTA-001-I / #40 Complete on Ubuntu; macOS/Windows verification explicitly residual  
Parallel work: other D/F iterations unchanged; #43 flaky fixture independently tracked.

## Goal and validation

Produce actual runner evidence that the generation state transaction lock and replacement work for multiple independent processes on macOS and Windows, not just Ubuntu. Add a scoped workflow with separate jobs for each OS, executing only `laputa-media` generation-state unit tests under the workspace-pinned Rust 1.95 toolchain.

## Hard boundaries

No live provider calls, no mutation of imported originals, no GUI functionality, no unrelated CI suite refactor. The workflow starts when generation store/domain paths or its own YAML change; optional manual dispatch is enabled.

## Risk / unresolved assumptions

The earlier portability workflow failed before Job creation (zero jobs). Incremental isolation identified the unquoted inline `cargo ... generation_store::tests::` command as the triggering YAML configuration; a literal block is now required. Runner smoke successfully scheduled both macOS and Windows. This iteration must distinguish an invalid workflow from a real macOS/Windows test failure. Do not mark platform coverage complete without green jobs for both.

## Evidence log

- Baseline `adabf39fc75da512f131d002fdaa7f978a3ff2b2`; new workflow and governance plan in PR for Issue #44.
- Workflow registration: confirmed by runner smoke at #44884672126, both macOS and Windows jobs completed successfully.
- A/B workflow diagnosis: inline `run: cargo ... generation_store::tests:: ...` failed before job allocation; using a literal YAML block `run: |` allowed scheduling on both runners.
- macOS generation-store test **PASS 9/9**, job [136788137609](https://github.com/wjhuang88/Laputa/actions/runs/44884959371/job/136788137609) at `050fe8699973fc9d26d3b82447c55abf29fee7c6`.
- Windows generation-store test **PASS 9/9**, job [136788137461](https://github.com/wjhuang88/Laputa/actions/runs/44884959371/job/136788137461) at same SHA, including the spawned cross-process writer test.
- Exact-head PR/push portability CI `44884959371`/`44884955085` and baseline CI `44884959301`/`44884955070`: **SUCCESS** at same SHA.
- Single-maintainer review [#6004147549](https://github.com/wjhuang88/Laputa/pull/46#pullrequestreview-6004147549): COMMENT self-review, **not** independent approval.
- Normal two-parent main sync `273e3718fbe633eae02c44663c959392b23201a3`: current-head macOS **9/9** and Windows **9/9**, [PR CI #46025102287](https://github.com/wjhuang88/Laputa/actions/runs/46025102287) SUCCESS. Same-head [baseline PR CI #46025102331](https://github.com/wjhuang88/Laputa/actions/runs/46025102331) and [push CI #46025090671](https://github.com/wjhuang88/Laputa/actions/runs/46025090671) SUCCESS.
- [PR #46](https://github.com/wjhuang88/Laputa/pull/46) merged as `a96e971b412579b7c5ba6a02a2605ee0a2c3f97a`.
- Post-merge main checks: [portability #46026645868](https://github.com/wjhuang88/Laputa/actions/runs/46026645868) and [baseline #46026645696](https://github.com/wjhuang88/Laputa/actions/runs/46026645696) **MUST be verified green before this closeout PR merges**.
- Final disposition: **complete** for target GitHub-hosted macOS/Windows local filesystems once closeout CI gates succeed. Network/filesystem variants and live Runway service remain unrun; unrelated flaky fixture #43 is Open.
