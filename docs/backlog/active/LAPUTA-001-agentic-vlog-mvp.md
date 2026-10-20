# LAPUTA-001 — Agentic Consumer Vlog MVP

Type: Epic  
Status: Refined  
Requirement baseline: `docs/reference/PRODUCT-REQUIREMENTS-v0.1.md`

## Outcome

Prove that a normal desktop user can add personal media, describe a desired short video, receive a Talos-driven first cut, make localized revisions through selection plus natural language/direct manipulation, and export a playable video without learning AI workflow or professional NLE concepts.

## Why this is an Epic

The result spans multiple independently verifiable outcomes: project/media safety, understanding, Agent control, first-cut timeline, preview UX, generative media, revision semantics, critic/repair, persistence and export. Treating all of them as one executable Story would conceal partial delivery.

## Constraints

Hard constraints and assumptions are owned by `docs/reference/PRODUCT-REQUIREMENTS-v0.1.md`. Child Stories must not weaken original-media immutability, deterministic edit scope, or asynchronous artifact validity.

## Child summary

| Child | Outcome | Status | Depends on | Iteration |
| --- | --- | --- | --- | --- |
| LAPUTA-001-A | Project creation + non-destructive mixed-media import | Complete | None | `2026-10-16-project-media-foundation.md`; `2026-10-17-gpui-project-import.md` |
| LAPUTA-001-B | Structured asset understanding via functional provider | In Progress | LAPUTA-001-A | `2026-10-20-asset-understanding-foundation.md` |
| LAPUTA-001-C | Talos-driven first-cut loop using Laputa tools | Proposed | LAPUTA-001-A, LAPUTA-001-B | - |
| LAPUTA-001-D | Playable preview + scene-strip representation | Proposed | LAPUTA-001-C | - |
| LAPUTA-001-E | Generative derived-media job/provider path | Proposed | LAPUTA-001-A, LAPUTA-001-C | - |
| LAPUTA-001-F | Selection-scoped revisions, direct manipulation and locks | Proposed | LAPUTA-001-D | - |
| LAPUTA-001-G | Structured critic + bounded repair loop | Proposed | LAPUTA-001-C, LAPUTA-001-D | - |
| LAPUTA-001-H | Reopen/continue + final MP4 export | Proposed | LAPUTA-001-A through LAPUTA-001-G as required for integrated path | - |

Story A is dependency-closed: automated branch/PR/main validation passed and the previously outstanding real-desktop import/reopen smoke was confirmed by the user on 2026-10-20. Story B is now the only selected implementation slice; First Cut work remains blocked on B.

## Epic completion condition

- All required child outcomes are Complete, or a scope decision explicitly removes/replaces a child and synchronizes this table plus the product requirement baseline.
- A user-reachable integration scenario proves import → one intent → playable first cut → localized revision → export.
- Original fixture media hashes are unchanged after the full scenario.
- Provider/model choices remain absent from the normal creation flow.
- Validation evidence is recorded in the iteration(s) that deliver the Epic.
