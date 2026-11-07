# LAPUTA-001-F — User can revise selected content with simple language and direct actions

Parent: LAPUTA-001  
Type: Product Story  
Status: In Progress — F1 merged, F2 Talos revision slice under review; Story acceptance pending D/GUI/variant  
Dependencies: LAPUTA-001-D  
Decision links: `docs/decisions/ADR-0003-agent-first-consumer-interaction.md`

As a normal user, I want to point at the part I dislike and say what should change, so that I can refine the video without learning editing commands.

## Execution state — 2026-11-06

**F1 (technical-only) merged by PR #31 (main `ca360903`, CI #42183222768 success)** under [2026-11-06 selection revision iteration](../../iterations/2026-11-06-selection-revision-core.md). `LAPUTA-001-D / #5` remains Open for **MA-D-01 real-desktop human acceptance**. Its production Scene Strip and Selection code are already merged; F1 may depend on those exact technical contracts **without claiming human acceptance of D**. This is a bounded foundation exception under the iteration SOP; all Story F user acceptance remains unfulfilled until product GUI/Talos wiring.

F1 implements:
- typed scope precedence (explicit overrides selected Scene; no Selection defaults Project; stale ID rejects);
- validated non-destructive reorder, trim, delete and persisted retain/lock for canonical `FilmState`, with backward-compatible schema-v1 read;
- project-local `FilmStore` edit-and-save path, fixture tests for rollback, reopen, original bytes and lock invariants;
- `WorkspaceState` Selection → revision scope projection, intentionally distinct from playback current Scene.

**F2 (2026-11-07):** A bounded Talos `timeline.get` / `timeline.edit` runtime slice is being implemented under [F2 iteration](../../iterations/2026-11-07-talos-selection-revision.md). It currently supports staged single-turn natural-language **shorten** and **delete selected** operations with explicit whole-video override, model-invalid scope rejection and post-success single-commit semantics. The initial 20% shortening is a tracked UX assumption, not a full general-language semantic editor.

**Remaining F3/F4:** Scene Strip direct GUI editing/preview refresh, product command-input wiring, variant retry, real product interaction and MA-F-01…04. The F1 core alone does **not** fulfill the Given/When/Then user-facing Story acceptance.

## Scope

- Deterministic selection-scope resolution.
- Natural-language revisions through Talos.
- Direct reorder, trim and delete for scene-strip items.
- Lock/retain state respected by later automatic revisions.
- “Try another” for generated variants without provider/model choice.

## Out of scope

- Multi-track editing, keyframes, advanced mask/effect inspector.

## Acceptance

- Given one scene is selected  
  When the user says “make this shorter”  
  Then only that selected scope is the default edit target.
- Given a scene is selected  
  When the user says “make the whole video warmer”  
  Then explicit whole-video scope overrides selection.
- Given a scene is locked  
  When a later global autonomous revision runs  
  Then the locked scene is not changed unless the user explicitly overrides/unlocks it.
- Given scenes A and B  
  When the user drags B before A  
  Then project/timeline order changes accordingly without requiring an Agent turn.

## Future manual verification (not yet executable)

**MA-F-01 through MA-F-04: FUTURE / NOT READY.** Selection-scoped language mutation, direct reorder/trim/delete, lock/retain semantics, and derived “Try another” require real product GUI validation **after** the corresponding implementation merges. These are **not** Story D's existing Selection-context smoke and must not be pre-checked. See [manual acceptance ledger](../../acceptance/MANUAL-ACCEPTANCE.md).

## Minimum validation

Scope-resolution tests plus GUI integration smoke checks for select/reorder/trim/delete/lock.
