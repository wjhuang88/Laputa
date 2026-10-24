# LAPUTA-001-D — User can preview and inspect the first cut as a scene strip

Parent: LAPUTA-001  
Type: Product Story  
Status: In Progress  
Dependencies: LAPUTA-001-C  
Decision links: `docs/decisions/ADR-0003-agent-first-consumer-interaction.md`

As a normal user, I want to watch the first cut and see its ordered scenes without learning tracks, so that I can understand what the AI made and choose what to change.

## Delivery sequencing

The current implementation begins with a **Film Workspace visual baseline checkpoint** before the remaining playback/selection behavior. The approved Simplified-Chinese `docs/design/assets/v0.1/03-workspace.jpg` is the high-fidelity target for this checkpoint.

This sequencing change does not relax Story acceptance. The Story remains open until the behavioral criteria below are satisfied.

## Scope

- High-fidelity Film Workspace visual baseline matching the approved hierarchy, density and Nordic flat visual language.
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

## Visual checkpoint gate

Before proceeding deeply into behavior, one production GPUI Film Workspace page should be reviewed against `03-workspace.jpg` and accepted as the visual tone baseline. It should prioritize Stage dominance, restrained side rails, a shallow Scene Strip, Simplified-Chinese copy, generous whitespace and large flat surfaces.

## Minimum validation

Visual baseline review + render fixture + GUI interaction smoke test; selection-context unit/integration tests.
