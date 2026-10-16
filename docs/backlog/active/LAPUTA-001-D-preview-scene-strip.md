# LAPUTA-001-D — User can preview and inspect the first cut as a scene strip

Parent: LAPUTA-001  
Type: Product Story  
Status: Proposed  
Dependencies: LAPUTA-001-C  
Decision links: `docs/decisions/ADR-0003-agent-first-consumer-interaction.md`

As a normal user, I want to watch the first cut and see its ordered scenes without learning tracks, so that I can understand what the AI made and choose what to change.

## Scope

- Deterministic preview render from timeline state.
- Play/pause/seek preview surface.
- Ordered selectable scene/clip strip.
- Current selection propagated as command context.

## Out of scope

- Default multi-track NLE, keyframe editor, effect inspector, professional scopes.

## Acceptance

- Given a valid first-cut timeline  
  When preview is requested  
  Then the user can play/seek the rendered result and see corresponding ordered scene-strip items.
- Given one scene-strip item is selected  
  When a later natural-language command is submitted without explicit scope  
  Then the application context marks that selection as the default edit target.

## Minimum validation

Render fixture + GUI interaction smoke test; selection-context unit/integration tests.
