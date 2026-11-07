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
| LAPUTA-001-B | Structured asset understanding via functional provider | Complete | LAPUTA-001-A | `2026-10-20-asset-understanding-foundation.md` |
| LAPUTA-001-C | Talos-driven first-cut loop using Laputa tools | Complete | LAPUTA-001-A, LAPUTA-001-B | `2026-10-21-talos-first-cut.md`; Issue #4 |
| LAPUTA-001-D | Playable preview + scene-strip representation | In Progress — MA-D-01 human gate | LAPUTA-001-C | `2026-10-23-workspace-visual-baseline.md`; Issue #5 |
| LAPUTA-001-E | Generative derived-media job/provider path | Complete | LAPUTA-001-A, LAPUTA-001-C | `2026-10-31-generative-media-foundation.md`; Issue #6 |
| LAPUTA-001-F | Selection-scoped revisions, direct manipulation and locks | In Progress — F1 merged, F2 Talos tool under review | LAPUTA-001-D (merged contracts; MA-D-01 pending) | `2026-11-06-selection-revision-core.md`; Issue #7 |
| LAPUTA-001-G | Structured critic + bounded repair loop | Proposed | LAPUTA-001-C, LAPUTA-001-D | - |
| LAPUTA-001-H | Reopen/continue + final MP4 export | Proposed | LAPUTA-001-A through LAPUTA-001-G as required for integrated path | - |

Stories A/B/C/E are Complete and closed. Story A includes user-confirmed real-desktop import/reopen evidence. Story D's canonical Film Workspace is integrated and automated product CI passed, but **MA-D-01 real-desktop human acceptance remains pending**, so Issue #5 stays open. F1 technical core is merged and F2 Tool/Runtime is in progress (not a completed Story F); G/H remain proposed. F, G, H and Epic manual walkthroughs are not yet suitable for completion. Runway/Gemini/Talos paid or credentialed live smokes remain optional, unrun residuals rather than invented proof.

## Human acceptance handoff

See the central [manual acceptance ledger](../../acceptance/MANUAL-ACCEPTANCE.md) for **current required MA-D-01**, future MA-F/MA-H/MVP walkthroughs, and credential-gated optional smokes. Parent Epic closure requires the complete user-reachable integration path; automated fixture results alone do not assert a user actually completed that path. Actual user evidence must be recorded before human-gated boxes are checked.

## Epic completion condition

- All required child outcomes are Complete, or a scope decision explicitly removes/replaces a child and synchronizes this table plus the product requirement baseline.
- A user-reachable integration scenario proves import → one intent → playable first cut → localized revision → export.
- Original fixture media hashes are unchanged after the full scenario.
- Provider/model choices remain absent from the normal creation flow.
- Validation evidence is recorded in the iteration(s) that deliver the Epic.
