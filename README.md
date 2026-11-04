# Laputa

[English](README.md) | [简体中文](README.zh-CN.md)

Laputa is an agentic desktop video-creation product for ordinary users. It turns personal photos and videos into a finished Vlog through one primary interaction loop:

> Add media → describe the result → review the first cut → select and revise → export.

Laputa uses [Talos](https://github.com/wjhuang88/talos) as its embedded Agent Loop runtime. The product domain, project state, media tools, timeline, provider abstractions, rendering, and GUI remain owned by Laputa.

## Current status

Stories **A/B/C/E are Complete/Closed**, including Story A's real-desktop import/reopen smoke (user-confirmed on 2026-10-20) and Story E's deterministic Tool-to-materialization path. Story **D / Issue #5** has a real canonical GPUI Film Workspace with preview, Scene Strip and Selection merged and automated tests passing; its **only remaining closure gate** is real-desktop **human interaction acceptance (MA-D-01)**. Stories F/G/H are proposed.

See the **[manual acceptance ledger](docs/acceptance/MANUAL-ACCEPTANCE.md)** for the pending real-desktop walkthrough, future Story F/H checks, completed Story A evidence, and optional credential-gated Gemini/Talos/Runway live API smokes. Do not treat automated Xvfb smoke as human acceptance.

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

Asset understanding, Talos First Cut, generative-media foundations, and the canonical playable Preview/Scene Strip are implemented. Selection-scoped editing/drag/trim/lock (F), Critic/repair (G), and final export (H) are later Stories; Story D's last real-desktop acceptance remains open.

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
