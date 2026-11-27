# Laputa Product Backlog

This file is the compact prioritization surface. Execution details live in item files under `active/`; completed/superseded work moves to `archive/` with history preserved.

## Required reads

Before implementing current MVP work, read:

- `docs/reference/PRODUCT-REQUIREMENTS-v0.1.md`
- `docs/reference/ARCHITECTURE.md`
- the selected Story file
- any ADRs listed by that Story

## Manual acceptance

The canonical human-test checklist is [`docs/acceptance/MANUAL-ACCEPTANCE.md`](../acceptance/MANUAL-ACCEPTANCE.md). **Story D / #5 has two open gates: MA-D-VIS-01 visual FAIL (rework required) and MA-D-01 interaction acceptance pending.** Story F/H and Epic walkthroughs are future/not-ready; external provider live smoke is optional and credential-gated. Story A's prior manual smoke is already recorded as Done.

## Active

| ID | Type | Outcome | Status | Decision Context | Required Reads |
| --- | --- | --- | --- | --- | --- |
| LAPUTA-001 | Epic | Prove the end-to-end agentic consumer Vlog MVP | Refined | Validate the simple “add → intent → first cut → revise → export” interaction before local-model deployment | `active/LAPUTA-001-agentic-vlog-mvp.md`, product requirements, architecture |
| LAPUTA-001-D | Story | User can preview and inspect the first cut as a scene strip | In Progress — visual FAIL; rework planned | MA-D-VIS-01 failed on 2026-11-16; visual rework and separate MA-D-01 interaction acceptance required | `active/LAPUTA-001-D-preview-scene-strip.md`, ADR-0003, `../design/UI-UX-VISION-v0.1.md` |
| LAPUTA-001-F | Story | User can revise selected content with language/direct manipulation and locks | In Progress — F1/F2/F3a merged, F3b/F4 pending | Scene buttons, save/reopen and preview refresh merged in #34; true drag, live language input, variants and human acceptance pending; D MA-D-01 still open | `active/LAPUTA-001-F-selection-revision.md`, ADR-0003 |
| LAPUTA-001-G | Story | Laputa can critique a preview and apply bounded repair | Proposed | Requires C+D and analysis provider | `active/LAPUTA-001-G-critic-repair.md`, ADR-0001, ADR-0002 |
| LAPUTA-001-H | Story | User can reopen a project and export a valid final MP4 | Proposed | Integration closure across A-G | `active/LAPUTA-001-H-persistence-export.md`, product requirements |

## Completed

| ID | Type | Outcome | Closed | Evidence |
| --- | --- | --- | --- | --- |
| LAPUTA-001-A | Story | User can create a project and safely import mixed media | 2026-10-20 | `archive/LAPUTA-001-A-project-media-import.md`; Issue #2; PR #12 |
| LAPUTA-001-B | Story | Imported assets can be structurally analyzed through a functional provider | 2026-10-20 | `archive/LAPUTA-001-B-asset-understanding.md`; Issue #3; PR #13 |
| LAPUTA-001-C | Story | Talos can produce a first-cut project state using Laputa tools | 2026-10-22 | `archive/LAPUTA-001-C-talos-first-cut.md`; Issue #4; PR #14 |
| LAPUTA-001-E | Story | Talos can request generative derived media without vendor coupling | 2026-11-04 | `archive/LAPUTA-001-E-generative-media.md`; Issue #6; PRs #23–#28; post-merge CI `41649040890` |
| LAPUTA-001-I | Technical defect Story | Concurrent-safe durable generation state updates (Ubuntu-tested; portability #44 residual) | 2026-11-21 | `archive/LAPUTA-001-I-generation-state-concurrency.md`; Issue #40; PR #42; push CI `44457205819`, PR CI `44457211375`; main CI `44863056584` gated at closeout |
| LAPUTA-001-J | Technical verification Story | macOS/Windows local-filesystem generation-state locking and replacement verified | 2026-11-27 | `archive/LAPUTA-001-J-generation-state-portability.md`; Issue #44; PR #46; macOS+Windows CI `46025102287`; baseline CI `46025102331`, `46025090671`; post-merge CI `46026645868`/`46026645696` gated at closeout |

## Later

Later ideas must remain in roadmap/proposals until refined into implementable Stories. Do not start them directly from this section.
