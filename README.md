# Laputa

[English](README.md) | [简体中文](README.zh-CN.md)

Laputa is an agentic desktop video-creation product for ordinary users. It turns personal photos and videos into a finished Vlog through one primary interaction loop:

> Add media → describe the result → review the first cut → select and revise → export.

Laputa uses [Talos](https://github.com/wjhuang88/talos) as its embedded Agent Loop runtime. The product domain, project state, media tools, timeline, provider abstractions, rendering, and GUI remain owned by Laputa.

## Current status

The product, governance, V0.1 requirement, and UI/UX visual baselines are initialized. `LAPUTA-001-A` remains **In Progress**: project persistence, non-destructive mixed-media import, the media-probe boundary, automated safety tests, and the GPUI project-create/open, system file-picker import, and Asset Browser paths are implemented.

The one required Story A evidence row still missing is a real graphical-desktop smoke proving **import → close/relaunch → reopen the same project → imported inventory remains visible**. A successful headless CI build is intentionally not treated as that manual evidence.

### Desktop GUI

The consumer desktop entrypoint is the single explicitly declared `laputa-desktop` binary, gated by the `desktop-ui` feature.

Launch the project home:

```sh
cargo run -p laputa-app --features desktop-ui --bin laputa-desktop
```

Create a project directly:

```sh
cargo run -p laputa-app --features desktop-ui --bin laputa-desktop -- \
  create ./demo-project "My Vlog"
```

Open an existing project directly:

```sh
cargo run -p laputa-app --features desktop-ui --bin laputa-desktop -- \
  open ./demo-project
```

Desktop import uses the native multi-file picker and reuses `ffprobe` through the existing `ProjectStore` import path, so install FFmpeg before importing media. Import/probe work runs off the UI thread. Successful imports update the Asset Browser from the committed `ProjectManifest`; failures preserve the visible project inventory and surface a recoverable error. Imported originals remain read-only; project metadata, caches, and future derivatives live under `<project>/.laputa/`.

The first Asset Browser intentionally shows media kind, filename, and available duration/dimension metadata instead of expanding Story A into thumbnail-generation work. The visual hierarchy follows the [Laputa V0.1 UI/UX baseline](docs/design/UI-UX-VISION-v0.1.md).

Media AI understanding, the Talos First Cut, generated media, playable Preview, real Scene Strip editing, Critic, and final export interaction remain later Stories in the `LAPUTA-001` A→H chain.

## Product principles

- Consumer-first: normal users should not need to understand models, providers, workflows, tracks, or generation parameters.
- Agent-driven: Talos decides how to reach the user's creative intent by invoking coarse-grained Vlog tools.
- Direct manipulation where it is clearer: selection, ordering, trimming, playback, and locking remain immediate GUI actions.
- Functional provider boundaries: providers are abstracted by capability, not by vendor.
- Non-destructive editing: original media is immutable; edits and generated assets are separate artifacts.
- Open-source-first evolution: external APIs may bootstrap V0.1, but provider boundaries must allow later local/open implementations.

## Repository map

- `crates/laputa-app`: desktop application shell and GPUI product entrypoint.
- `crates/laputa-domain`: project-world domain model.
- `crates/laputa-agent`: Talos runtime composition and Laputa tools.
- `crates/laputa-provider`: functional provider contracts.
- `crates/laputa-media`: deterministic media operations, project persistence, and non-destructive import.
- `crates/laputa-timeline`: timeline IR and render-facing editing model.
- `docs/design/UI-UX-VISION-v0.1.md`: approved V0.1 consumer visual/interaction direction.
- `docs/reference/PRODUCT-REQUIREMENTS-v0.1.md`: approved V0.1 product requirement baseline.
- `docs/backlog/active/LAPUTA-001-agentic-vlog-mvp.md`: first implementation Epic.
- `AGENTS.md`: Agent-facing project rules and task router.

See [docs/README.md](docs/README.md) for the governance and engineering document map.
