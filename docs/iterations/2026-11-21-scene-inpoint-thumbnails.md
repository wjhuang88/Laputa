# Iteration — Story D scene video in-point thumbnails

Status: technical code merged PR #47, exact-head CI success; independent post-merge main CI tracked separately; human visual and interaction gates pending
Date: 2026-11-21
Selected: LAPUTA-001-D / Issue #5 (bounded post-V1 rendering slice)
Baseline: main `adabf39fc75da512f131d002fdaa7f978a3ff2b2`; V1 PR #39 already merged, exact-head and post-merge CI successful.

## Goal

A video cut's storyboard card and right-hand selected-scene preview must reflect **its own validated source start time**, not the imported video's first frame. Keep Asset Browser previews source-level. Reuse the project-local FFmpeg thumbnail cache, a bounded background queue, project switch fencing and existing FilmState/Revision flows.

## Contracts and checks

- Source-specific cache key includes immutable imported asset SHA-256 and the validated video `start_ms`. FFmpeg accurate seek is applied after input. Image scenes continue using existing image thumbnail.
- Two scenes referencing the same video at different offsets must produce different derivative paths and visibly distinct frames; edits changing in-point reproject with new cache key (without reopening).
- GPU thumbnails and in-flight work are keyed by scene identity + asset hash + start_ms, and obsolete async results cannot replace the current scene's image. All media originals are input-only.
- Deterministic image/video fixture tests cover two offsets on one video, changed cache key, damaged-cache rebuild, source hashes unchanged and failed seek/no cache publication.
- UI product smoke checks actual video in-point cache generation, screenshot after selecting scene, and unchanged original SHA ledger. Exact-head fmt, full workspace, clippy, desktop tests and post-merge CI must be reviewed.
- No UI data model/revision semantic changes, no F3b/F4 feature development, no generated output/provider calls.

## Manual acceptance

Story D MA-D-VIS-01 is **FAIL** and MA-D-01 is **PENDING** until the user executes real desktop photographic-media acceptance. Six-scene FFmpeg synthetic screenshots are technical regressions, not a visual PASS. At future real-desktop acceptance, reuse a single 3+ second video as two different scenes, set one start to 0 and the other to at least 1 second, verify distinct visual content in the Scene Strip and selected right rail after save/reopen and after changing trim.

## Disposition

**Technical slice merged**, PR [#47](https://github.com/wjhuang88/Laputa/pull/47) squash commit `079e6e22b6b794a4eb133e704f97ca9e29045e22`. Exact-head source `423f58f8a4b60eabd1762bf27c7a22d5c69c995e`; push CI [#44885774237](https://github.com/wjhuang88/Laputa/actions/runs/44885774237), PR CI [#44885777859](https://github.com/wjhuang88/Laputa/actions/runs/44885777859), real GPUI product regression [#44885777770](https://github.com/wjhuang88/Laputa/actions/runs/44885777770) all **SUCCESS**. Independent post-merge main CI [#44886628771](https://github.com/wjhuang88/Laputa/actions/runs/44886628771) and product smoke [#44886628740](https://github.com/wjhuang88/Laputa/actions/runs/44886628740) must be checked from actual outcomes and are not pre-checked here. Original media hash evidence in the product artifact matched. Story D Issue #5 remains **Open**: MA-D-VIS-01 **FAIL**, MA-D-01 **PENDING**, conditional in-point human preflight **UNRUN**; F3b2 UI trim controls not merged.
