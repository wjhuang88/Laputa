# LAPUTA-001-C — Talos can create a first-cut project state through Laputa tools

Parent: LAPUTA-001  
Type: Product/API Story  
Status: Ready  
Dependencies: LAPUTA-001-A, LAPUTA-001-B (Complete)  
Decision links: `docs/decisions/ADR-0001-talos-runtime-boundary.md`, `docs/decisions/ADR-0002-functional-provider-abstraction.md`  
Iteration: `docs/iterations/2026-10-21-talos-first-cut.md`

As a person who does not want to edit manually,  
I want Laputa to turn one natural-language creative request into a first cut,  
so that I can start from a result rather than a blank timeline.

## Scope

- Embed the already-pinned `talos-runtime` through Laputa's composition layer; no Talos domain fork.
- Introduce a small durable Laputa-owned FilmState containing creative intent, story plan and ordered timeline state.
- Persist FilmState atomically under project-local `.laputa/` state without changing Story A `project.json` schema v1.
- Expose the minimum coarse-grained Talos tools needed for the initial First Cut:
  - read-only `project.inspect` over imported Assets + canonical Ready analyses;
  - atomic `film.commit_first_cut` over a validated model draft.
- Derive human-readable `IntentSpec` from the user's request as part of the committed state.
- Talos can inspect analyzed assets and commit a valid initial story/timeline state.
- Translate Talos runtime/tool events into compact Laputa progress for the app without exposing low-level provider workflow controls.

## Out of scope

- Generative media.
- Critic auto-repair.
- Playable preview/rendering and Scene Strip UI (Story D).
- Granular manual timeline editing / selection revisions (Story F).
- Multi-Agent architecture.
- Provider/model/workflow selectors.
- Implementing the full eventual V0.1 semantic tool catalog before later Stories require it.

## Acceptance

- [ ] Given a project with analyzed fixture media, when the user enters one creative request, Talos can use Laputa tools to produce and persist a valid FilmState with Intent + Story + ordered Timeline.
- [ ] The user is not required to choose a provider, model, workflow graph, or track layout.
- [ ] Invalid Asset references, invalid video source ranges, or otherwise invalid First Cut drafts are rejected before durable state changes.
- [ ] Given a First Cut mutation tool failure, Talos receives an explicit tool error and no successful FilmState is invented or partially persisted.
- [ ] Reopening the project restores the committed FilmState while the original Story A Project/Asset manifest remains compatible at schema v1.
- [ ] App integration can observe compact First Cut progress without receiving provider/API orchestration as its normal progress contract.

## Canonical boundary

The initial durable editorial state is intentionally small:

- `IntentSpec`: original request, normalized summary, optional target duration, tone tags;
- `StoryPlan`: summary + ordered semantic beats;
- `Timeline`: ordered scenes referencing imported Asset IDs and validated source selections;
- Laputa-generated stable IDs for story beats/scenes.

Provider output, Talos internal messages/tool-call representation, model names and low-level media-engine details are not canonical FilmState.

## Minimum validation

- FilmState validation and persistence/reopen tests.
- `project.inspect` canonical snapshot test.
- `film.commit_first_cut` success + invalid input / atomic failure tests.
- Deterministic fake Talos LanguageModel Agent/tool integration test proving inspect → commit → persisted state.
- Progress-projection test proving app-facing phases do not expose provider/model/API detail.
- Optional configured live Talos-provider smoke when credentials are available; lack of credentials is recorded explicitly and never replaced by fabricated evidence.

## State owners to synchronize

- `docs/backlog/PRODUCT-BACKLOG.md`
- parent `LAPUTA-001` summary
- `docs/iterations/2026-10-21-talos-first-cut.md`
- Issue #4
