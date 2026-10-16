# LAPUTA-001-G — Laputa can critique a preview and perform bounded repair

Parent: LAPUTA-001  
Type: Product/API Story  
Status: Proposed  
Dependencies: LAPUTA-001-C, LAPUTA-001-D  
Decision links: `docs/decisions/ADR-0001-talos-runtime-boundary.md`, `docs/decisions/ADR-0002-functional-provider-abstraction.md`

As a user who expects a useful automatic result, I want Laputa to notice obvious problems in its own preview and repair them when safe, so that fewer manual corrections are needed.

## Scope

- Structured critique of rendered proxy through understanding provider.
- Issues reference time range/scene and severity/type.
- Talos receives issues and may revise through existing Tools.
- Bounded repair policy by retries/cost/time and explicit unresolved issue reporting.

## Out of scope

- Claiming subjective artistic perfection, endless self-reflection loops, multi-Agent debate architecture.

## Acceptance

- Given a preview with a fixture-known pacing/transition defect  
  When critique runs  
  Then a structured issue references the affected scene/range.
- Given a repair is safe and within budget  
  When Talos applies a revision  
  Then a new preview/project revision is produced without mutating the prior/original assets.
- Given the retry/budget bound is reached  
  When issues remain  
  Then the loop stops and exposes unresolved issues instead of retrying indefinitely.

## Minimum validation

Critique schema tests, bounded-loop tests, and one end-to-end proxy → critique → revision fixture.
