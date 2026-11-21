# Iteration — Generation State Portability Verification

Status: In Progress — CI workflow registration and platform tests pending  
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
- macOS generation-store test: **QUEUED / NOT YET VERIFIED** at #44884899173.
- Windows generation-store test: **QUEUED / NOT YET VERIFIED** at #44884899173.
- Exact-head PR/Push CI and single-maintainer review: **PENDING**.
- Result: **partial** until independent OS results and post-merge CI are recorded.
