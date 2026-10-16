# ADR-0001 — Embed Talos as the Agent runtime boundary

## Context

Laputa needs a model-driven Observe/Reason/Act loop, tool calling, streaming events, cancellation/background work and session behavior. Talos already provides an embeddable Rust runtime facade and is maintained as a separate general Agent project.

## Constraint Decomposition

| Constraint | Type | Source | Can Change? |
| --- | --- | --- | --- |
| Laputa needs an Agent Loop capable of iterative tool use | Hard | Product architecture need | No |
| Reuse Talos rather than building another loop | Soft | Project direction | Yes |
| Talos embedding surface remains suitable as Laputa grows | Assumption | Current Talos SDK state | Maybe |
| Laputa domain must remain independently evolvable | Hard | Product boundary | No |

## Reasoning

Reimplementing the loop would duplicate a large generic concern and delay product validation. Coupling Laputa directly to Talos internal Agent crates or UI would make domain evolution depend on unstable internals. The supported `talos-runtime` facade is the narrowest reuse boundary.

## Decision

Laputa embeds Talos through `talos-runtime`. Talos owns generic Agent runtime behavior; Laputa owns project world, Vlog tools, media/provider/timeline logic and GUI. Laputa-specific concepts must not be added to Talos solely for application convenience.

The initial dependency is pinned to Talos commit `0ef5a532930642d730bcc6d974f2a79605efaaee`; pin changes are explicit dependency updates with integration validation.

## Reversal Trigger

Revisit if Talos cannot support a required product behavior without repeated application-specific core changes, if the embedding contract becomes unavailable, or if another runtime materially reduces complexity while preserving tool/session/cancellation requirements.
