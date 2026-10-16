# LAPUTA-001-F — User can revise selected content with simple language and direct actions

Parent: LAPUTA-001  
Type: Product Story  
Status: Proposed  
Dependencies: LAPUTA-001-D  
Decision links: `docs/decisions/ADR-0003-agent-first-consumer-interaction.md`

As a normal user, I want to point at the part I dislike and say what should change, so that I can refine the video without learning editing commands.

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

## Minimum validation

Scope-resolution tests plus GUI integration smoke checks for select/reorder/trim/delete/lock.
