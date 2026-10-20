# Laputa Product Backlog

This file is the compact prioritization surface. Execution details live in item files under `active/`; completed/superseded work moves to `archive/` with history preserved.

## Required reads

Before implementing current MVP work, read:

- `docs/reference/PRODUCT-REQUIREMENTS-v0.1.md`
- `docs/reference/ARCHITECTURE.md`
- the selected Story file
- any ADRs listed by that Story

## Active

| ID | Type | Outcome | Status | Decision Context | Required Reads |
| --- | --- | --- | --- | --- | --- |
| LAPUTA-001 | Epic | Prove the end-to-end agentic consumer Vlog MVP | Refined | Validate the simple “add → intent → first cut → revise → export” interaction before local-model deployment | `active/LAPUTA-001-agentic-vlog-mvp.md`, product requirements, architecture |
| LAPUTA-001-C | Story | Talos can produce a first-cut project state using Laputa tools | Proposed | A+B are Complete; next selectable slice, but not started by the B iteration | `active/LAPUTA-001-C-talos-first-cut.md`, ADR-0001, ADR-0002 |
| LAPUTA-001-D | Story | User can preview and inspect the first cut as a scene strip | Proposed | Requires C timeline state | `active/LAPUTA-001-D-preview-scene-strip.md`, ADR-0003 |
| LAPUTA-001-E | Story | Talos can request generative derived media without vendor coupling | Proposed | Requires artifact/job contract and C tool path | `active/LAPUTA-001-E-generative-media.md`, ADR-0002 |
| LAPUTA-001-F | Story | User can revise selected content with language/direct manipulation and locks | Proposed | Requires D selection/scene-strip interaction | `active/LAPUTA-001-F-selection-revision.md`, ADR-0003 |
| LAPUTA-001-G | Story | Laputa can critique a preview and apply bounded repair | Proposed | Requires C+D and analysis provider | `active/LAPUTA-001-G-critic-repair.md`, ADR-0001, ADR-0002 |
| LAPUTA-001-H | Story | User can reopen a project and export a valid final MP4 | Proposed | Integration closure across A-G | `active/LAPUTA-001-H-persistence-export.md`, product requirements |

## Completed

| ID | Type | Outcome | Closed | Evidence |
| --- | --- | --- | --- | --- |
| LAPUTA-001-A | Story | User can create a project and safely import mixed media | 2026-10-20 | `archive/LAPUTA-001-A-project-media-import.md`; Issue #2; PR #12 |
| LAPUTA-001-B | Story | Imported assets can be structurally analyzed through a functional provider | 2026-10-20 | `archive/LAPUTA-001-B-asset-understanding.md`; Issue #3; PR #13 |

## Later

Later ideas must remain in roadmap/proposals until refined into implementable Stories. Do not start them directly from this section.
