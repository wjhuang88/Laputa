# ADR-0003 — Use an Agent-first consumer interaction model

## Context

Laputa targets ordinary users rather than professional editors. Traditional NLEs and AI workflow products expose many implementation choices that create ambiguity and decision burden.

## Constraint Decomposition

| Constraint | Type | Source | Can Change? |
| --- | --- | --- | --- |
| Ordinary users must be able to finish a video without professional editing concepts | Hard | Product goal | No |
| Default UI should avoid visible multi-track timeline | Soft | Current interaction strategy | Yes |
| Selection + language covers most revision scope needs | Assumption | Product hypothesis | Maybe |
| Direct manipulation is clearer for ordering/trimming | Soft | Interaction principle | Yes |

## Reasoning

The shortest understandable mental model is “add → say → watch → select and revise → export.” Natural language is best for semantic intent; direct manipulation is better for spatial/time actions. Selection supplies context and removes repeated clarification.

## Decision

The default product flow uses asset browser, central preview, scene strip, creative-intent summary and one natural-language command surface. Selection is the default edit scope; no selection means current video/project; explicit language overrides selection. Reorder/trim/delete/playback/lock are direct actions. Provider/model/workflow choice is hidden from the normal flow.

A professional multi-track editor is not part of the V0.1 default surface.

## Reversal Trigger

Revisit if usability evidence shows common required revisions cannot be expressed reliably with selection + language + scene-strip direct manipulation, or if users repeatedly need precise multi-track control to complete the target use cases.
