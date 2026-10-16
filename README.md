# Laputa

[English](README.md) | [简体中文](README.zh-CN.md)

Laputa is an agentic desktop video-creation product for ordinary users. It turns personal photos and videos into a finished Vlog through one primary interaction loop:

> Add media → describe the result → review the first cut → select and revise → export.

Laputa uses [Talos](https://github.com/wjhuang88/talos) as its embedded Agent Loop runtime. The product domain, project state, media tools, timeline, provider abstractions, rendering, and GUI remain owned by Laputa.

## Current status

The product baseline and governance are initialized. `LAPUTA-001-A` is now **In Progress**: the project-persistence, non-destructive mixed-media import, media-probe boundary, and automated safety-test foundation is implemented and CI-validated. The remaining Story A acceptance is the GPUI file-picker/drag-and-drop path, Asset Browser presentation, and desktop GUI smoke evidence.

The current runnable foundation intentionally uses a thin CLI so the storage/import contract can be validated before the consumer GUI is defined:

```sh
cargo run -p laputa-app -- create ./demo-project "My Vlog"
cargo run -p laputa-app -- import ./demo-project /path/to/photo.jpg /path/to/clip.mp4
cargo run -p laputa-app -- open ./demo-project
```

`import` uses `ffprobe` for deterministic media metadata. Install FFmpeg for this developer smoke path. Imported originals are referenced read-only; project metadata, caches, and future derivatives live under `<project>/.laputa/`.

## Product principles

- Consumer-first: normal users should not need to understand models, providers, workflows, tracks, or generation parameters.
- Agent-driven: Talos decides how to reach the user's creative intent by invoking coarse-grained Vlog tools.
- Direct manipulation where it is clearer: selection, ordering, trimming, playback, and locking remain immediate GUI actions.
- Functional provider boundaries: providers are abstracted by capability, not by vendor.
- Non-destructive editing: original media is immutable; edits and generated assets are separate artifacts.
- Open-source-first evolution: external APIs may bootstrap V0.1, but provider boundaries must allow later local/open implementations.

## Repository map

- `crates/laputa-app`: desktop application shell and current foundation CLI.
- `crates/laputa-domain`: project-world domain model.
- `crates/laputa-agent`: Talos runtime composition and Laputa tools.
- `crates/laputa-provider`: functional provider contracts.
- `crates/laputa-media`: deterministic media operations, project persistence, and non-destructive import.
- `crates/laputa-timeline`: timeline IR and render-facing editing model.
- `docs/reference/PRODUCT-REQUIREMENTS-v0.1.md`: approved V0.1 product requirement baseline.
- `docs/backlog/active/LAPUTA-001-agentic-vlog-mvp.md`: first implementation Epic.
- `AGENTS.md`: Agent-facing project rules and task router.

See [docs/README.md](docs/README.md) for the governance and engineering document map.
