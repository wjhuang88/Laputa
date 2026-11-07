# Laputa Product Backlog

This file is the compact prioritization surface. Execution details live in item files under `active/`; completed/superseded work moves to `archive/` with history preserved.

## Required reads

Before implementing current MVP work, read:

- `docs/reference/PRODUCT-REQUIREMENTS-v0.1.md`
- `docs/reference/ARCHITECTURE.md`
- the selected Story file
- any ADRs listed by that Story

## Manual acceptance

The canonical human-test checklist is [`docs/acceptance/MANUAL-ACCEPTANCE.md`](../acceptance/MANUAL-ACCEPTANCE.md). **Only MA-D-01 (Story D / #5) currently blocks a delivered Story on real-desktop user confirmation.** Story F/H and Epic walkthroughs are future/not-ready; external provider live smoke is optional and credential-gated. Story A's prior manual smoke is already recorded as Done.

## Active

| ID | Type | Outcome | Status | Decision Context | Required Reads |
| --- | --- | --- | --- | --- | --- |
| LAPUTA-001 | Epic | Prove the end-to-end agentic consumer Vlog MVP | Refined | Validate the simple “add → intent → first cut → revise → export” interaction before local-model deployment | `active/LAPUTA-001-agentic-vlog-mvp.md`, product requirements, architecture |
| LAPUTA-001-D | Story | User can preview and inspect the first cut as a scene strip | In Progress — final real-desktop acceptance | Canonical product binding is merged and automated product smoke is green; only real-desktop human interaction acceptance MA-D-01 remains | `active/LAPUTA-001-D-preview-scene-strip.md`, ADR-0003, `../design/UI-UX-VISION-v0.1.md` |
| LAPUTA-001-F | Story | User can revise selected content with language/direct manipulation and locks | In Progress — F1/F2 merged, F3/F4 pending | F1 (#31) and F2 (#32) technical checkpoints merged; D MA-D-01 and F GUI/variant/manual acceptance remain pending | `active/LAPUTA-001-F-selection-revision.md`, ADR-0003 |
| LAPUTA-001-G | Story | Laputa can critique a preview and apply bounded repair | Proposed | Requires C+D and analysis provider | `active/LAPUTA-001-G-critic-repair.md`, ADR-0001, ADR-0002 |
| LAPUTA-001-H | Story | User can reopen a project and export a valid final MP4 | Proposed | Integration closure across A-G | `active/LAPUTA-001-H-persistence-export.md`, product requirements |

## Completed

| ID | Type | Outcome | Closed | Evidence |
| --- | --- | --- | --- | --- |
| LAPUTA-001-A | Story | User can create a project and safely import mixed media | 2026-10-20 | `archive/LAPUTA-001-A-project-media-import.md`; Issue #2; PR #12 |
| LAPUTA-001-B | Story | Imported assets can be structurally analyzed through a functional provider | 2026-10-20 | `archive/LAPUTA-001-B-asset-understanding.md`; Issue #3; PR #13 |
| LAPUTA-001-C | Story | Talos can produce a first-cut project state using Laputa tools | 2026-10-22 | `archive/LAPUTA-001-C-talos-first-cut.md`; Issue #4; PR #14 |
| LAPUTA-001-E | Story | Talos can request generative derived media without vendor coupling | 2026-11-04 | `archive/LAPUTA-001-E-generative-media.md`; Issue #6; PRs #23–#28; post-merge CI `41649040890` |

## Later

Later ideas must remain in roadmap/proposals until refined into implementable Stories. Do not start them directly from this section.