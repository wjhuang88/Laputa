# LAPUTA-001-C — Talos can create a first-cut project state through Laputa tools

Parent: LAPUTA-001  
Type: Product/API Story  
Status: Complete  
Closed: 2026-10-22  
Issue: #4  
Decision links: `docs/decisions/ADR-0001-talos-runtime-boundary.md`, `docs/decisions/ADR-0002-functional-provider-abstraction.md`  
Iteration: `docs/iterations/2026-10-21-talos-first-cut.md`

## Outcome

A consumer request can now drive a deterministic Talos loop over Laputa-owned semantic tools and produce a validated durable first-cut state. Talos remains generic: it observes canonical project/analysis context, proposes one complete editorial draft, and asks Laputa to commit it atomically. Laputa owns the FilmState schema, IDs, validation, persistence and product-level progress projection.

## Delivered

- Durable `FilmState` schema v1 in `laputa-timeline`, containing `IntentSpec`, `StoryPlan`, semantic beats and an ordered scene timeline.
- Laputa-generated stable beat/scene UUIDs; the model supplies simple beat keys and imported Asset IDs instead of inventing canonical identifiers.
- `FilmStore` persistence at `.laputa/film.json`, independently validated from Story A `project.json` schema v1.
- Validation for non-empty intent/story/timeline state, unique IDs, valid beat references, imported Asset references, media-kind/source-shape compatibility and bounded video ranges.
- Read-only `project.inspect` Talos tool exposing imported moments plus canonical Ready analysis without vendor raw payloads.
- Atomic `film.commit_first_cut` Talos tool binding the original user request server-side and rejecting invalid drafts before durable mutation.
- `FirstCutRunner` composition using only the pinned `talos-runtime`, the two Laputa tools and an exact allow rule for `film.commit_first_cut`; no coding preset or broad Write permission is enabled.
- Product-level `FirstCutProgress`: Starting, Understanding moments, Finding the story, Building scenes, Completed/Failed.
- Progress derived from authoritative Talos ToolCall/ToolResult events rather than model text/reasoning/provider details.
- Success fence that reloads FilmState after runtime completion and requires a newly committed state whose original request matches the submitted request, preventing stale FilmState reuse from being reported as success.
- Deterministic scripted LanguageModel tests proving inspect → commit → persistence, expected progress ordering, invalid commit failure and old-state anti-reuse behavior.

## Acceptance

- [x] Given a project with analyzed fixture media, one creative request can drive Talos through Laputa tools to produce and persist a valid FilmState with Intent + Story + ordered Timeline.
- [x] The First Cut API requires no provider, model, workflow graph or track-layout choice from the user.
- [x] Invalid Asset references, invalid video source ranges and invalid editorial structures are rejected before successful durable state changes.
- [x] First Cut mutation failure is represented as an explicit Talos tool error; the runner does not invent success or accept an absent/old FilmState as the result of the turn.
- [x] FilmState can be reopened and revalidated while Story A `project.json` remains schema v1 and is not rewritten by FilmStore.
- [x] App integration can consume compact First Cut progress without provider/API orchestration as its normal progress contract.

## Evidence

- State/tool checkpoint PR CI `39808560339`: success (`fmt`, workspace `check`, Clippy with `-D warnings`, workspace tests, desktop-ui target and governance).
- Final runtime checkpoint push CI `39811272747`: success.
- Final runtime checkpoint PR CI `39811279902`: success with the same complete validation gate.
- PR #14 contains the implementation and closure history.
- `laputa-timeline` tests cover canonical draft conversion, invalid Asset/source-range rejection, FilmState reopen, `project.json` non-interference and invalid-state non-replacement.
- `laputa-agent::first_cut` tests cover canonical project inspection, successful atomic commit and explicit rejected commit.
- `laputa-agent::first_cut_runtime` tests cover deterministic Talos inspect → commit, product progress ordering, invalid commit failure and prevention of stale FilmState reuse.

## Live-provider residual

No configured live Talos language-model credential/fixture was supplied to this iteration, so an external live-provider smoke was **not executed** and no live-provider evidence is claimed. Normal CI remains credential-free and deterministic. A live smoke can be added/executed later against the same `FirstCutRunner` seam without changing FilmState or tool contracts.

## Boundary correction

The published iteration plan initially described FilmState persistence as an addition to `ProjectStore`. Implementation review showed that this would create a `laputa-media -> laputa-timeline` dependency even though FilmState is editorial/timeline truth. `ProjectStore` therefore remains the owner of Project/Asset/understanding state, while `FilmStore` is colocated with FilmState in `laputa-timeline` and writes the same project-local `.laputa/film.json` path. No new crate or project-manifest migration was introduced.

## Residuals intentionally not part of Story C

- Playable preview and Scene Strip UI (`LAPUTA-001-D`).
- Generative derived media (`LAPUTA-001-E`).
- Selection-scoped/manual timeline revision and Keep semantics (`LAPUTA-001-F`).
- Preview critic / bounded repair (`LAPUTA-001-G`).
- Final render/export integration (`LAPUTA-001-H`).
- Rich transitions/effects/audio-mix/generated-artifact/lock/critic metadata in FilmState until later Stories require them.
