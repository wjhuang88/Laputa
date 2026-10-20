# LAPUTA-001-A — User can create a project and safely import mixed media

Parent: LAPUTA-001  
Type: Product Story  
Status: Complete  
Closed: 2026-10-20  
Issue: #2  
Decision links: `docs/decisions/ADR-0003-agent-first-consumer-interaction.md`  
Design reference: `docs/design/UI-UX-VISION-v0.1.md`

## Outcome

A desktop user can create/open a Laputa project, import supported local photos/videos through the native picker, see persisted Asset inventory in the GPUI Asset Browser, and close/relaunch/reopen the project without re-importing originals. Original source media remains byte-for-byte unchanged.

## Delivered

- Versioned `ProjectManifest`, `ProjectId`, `AssetId`, `MediaKind`, technical metadata and immutable original Asset references.
- Project-local `.laputa/` state/cache/derivatives layout.
- Non-destructive mixed-media import with SHA-256 provenance, duplicate/type validation and atomic candidate-manifest commit semantics.
- `MediaProbe` boundary with `ffprobe` implementation.
- Feature-gated `laputa-desktop` GPUI application plus retained deterministic CLI path.
- Native project-folder create/open selection and native multi-file import selection.
- Background execution for blocking filesystem/ffprobe work.
- Asset Browser projection with `全部 / 照片 / 视频` filters and basic metadata.
- Persisted reopen state and recoverable import/probe failures.
- Renderer-independent `DesktopState` tests.

## Acceptance

- [x] New project + mixed media import displays imported items in Asset Browser.
- [x] Original fixture/source bytes remain unchanged.
- [x] Project metadata/cache/derivatives remain separate from source media.
- [x] Reopen restores Asset inventory without re-importing originals.
- [x] Real-desktop import + close/relaunch + reopen smoke completed and user-confirmed on 2026-10-20 as “符合预期”.

## Evidence

- Foundation CI `38860507105`: success.
- GPUI clean branch CI `39010083863`: success.
- PR #12 CI `39010653178`: success.
- Post-merge main CI `39099379963`: success.
- Implementation merge: PR #12, main commit `1c374942f4530a495860459d253aad49009b2024`.
- Iterations: `docs/iterations/2026-10-16-project-media-foundation.md`, `docs/iterations/2026-10-17-gpui-project-import.md`.

## Residuals intentionally not part of Story A

- Drag-and-drop import (picker satisfied the picker and/or drag-drop acceptance).
- Generated thumbnails/proxies beyond the minimal Asset Browser representation.
- AI media understanding, First Cut, Scene Strip, generation, revision and export.
