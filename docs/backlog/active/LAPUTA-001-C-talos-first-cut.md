# LAPUTA-001-C — Talos can create a first-cut project state through Laputa tools

Parent: LAPUTA-001  
Type: Product/API Story  
Status: Proposed  
Dependencies: LAPUTA-001-A, LAPUTA-001-B  
Decision links: `docs/decisions/ADR-0001-talos-runtime-boundary.md`, `docs/decisions/ADR-0002-functional-provider-abstraction.md`

As a person who does not want to edit manually,  
I want Laputa to turn one natural-language creative request into a first cut,  
so that I can start from a result rather than a blank timeline.

## Scope

- Embed pinned `talos-runtime` in `laputa-agent`.
- Project/asset/story/timeline semantic tools sufficient for a first cut.
- Human-readable `IntentSpec` derived from the user's request.
- Talos loop can inspect analyzed assets and commit a valid initial story/timeline project state.
- Tool/event progress reaches the app without exposing low-level provider workflow controls.

## Out of scope

- Generative media, critic auto-repair, full GUI timeline, multi-Agent architecture.

## Acceptance

- Given a project with analyzed fixture media  
  When the user enters one creative request  
  Then Talos can use Laputa tools to produce a valid first-cut timeline state  
  And the user is not required to choose a provider, model, workflow graph, or track layout.
- Given a tool call fails  
  When the failure reaches Talos  
  Then the loop receives an explicit structured failure and does not invent successful state.

## Minimum validation

Deterministic fake-provider Agent/tool integration test plus an optional configured live Talos provider smoke test.
