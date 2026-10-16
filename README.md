# Laputa

[English](README.md) | [简体中文](README.zh-CN.md)

Laputa is an agentic desktop video-creation product for ordinary users. It turns personal photos and videos into a finished Vlog through one primary interaction loop:

> Add media → describe the result → review the first cut → select and revise → export.

Laputa uses [Talos](https://github.com/wjhuang88/talos) as its embedded Agent Loop runtime. The product domain, project state, media tools, timeline, provider abstractions, rendering, and GUI remain owned by Laputa.

## Current status

This repository is an initialized product workspace. Governance, architecture boundaries, the initial product-requirement baseline, and the first MVP Epic are defined. End-to-end product implementation has not started yet.

## Product principles

- Consumer-first: normal users should not need to understand models, providers, workflows, tracks, or generation parameters.
- Agent-driven: Talos decides how to reach the user's creative intent by invoking coarse-grained Vlog tools.
- Direct manipulation where it is clearer: selection, ordering, trimming, playback, and locking remain immediate GUI actions.
- Functional provider boundaries: providers are abstracted by capability, not by vendor.
- Non-destructive editing: original media is immutable; edits and generated assets are separate artifacts.
- Open-source-first evolution: external APIs may bootstrap V0.1, but provider boundaries must allow later local/open implementations.

## Repository map

- `crates/laputa-app`: desktop application shell.
- `crates/laputa-domain`: project-world domain model.
- `crates/laputa-agent`: Talos runtime composition and Laputa tools.
- `crates/laputa-provider`: functional provider contracts.
- `crates/laputa-media`: deterministic media operations and artifact handling.
- `crates/laputa-timeline`: timeline IR and render-facing editing model.
- `docs/reference/PRODUCT-REQUIREMENTS-v0.1.md`: approved V0.1 product requirement baseline.
- `docs/backlog/active/LAPUTA-001-agentic-vlog-mvp.md`: first implementation Epic.
- `AGENTS.md`: Agent-facing project rules and task router.

See [docs/README.md](docs/README.md) for the governance and engineering document map.
