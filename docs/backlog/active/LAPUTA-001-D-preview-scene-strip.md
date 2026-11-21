# LAPUTA-001-D — User can preview and inspect the first cut as a scene strip

Parent: LAPUTA-001  
Type: Product Story  
Status: In Progress — visual acceptance FAIL; rework planned; MA-D-01 pending
Dependencies: LAPUTA-001-C  
Decision links: `docs/decisions/ADR-0003-agent-first-consumer-interaction.md`

As a normal user, I want to watch the first cut and see its ordered scenes without learning tracks, so that I can understand what the AI made and choose what to change.

## Current visual acceptance — 2026-11-16

**MA-D-VIS-01: FAIL.** 用户在 main `310efec` 的真实桌面验收中明确否定整体视觉还原。素材缩略图、场景卡片、布局和上下文面板均有明显差距。历史 PR #17 的接受记录保留，但不证明当前产品达到高保真要求。

[返工方案](../../iterations/2026-11-16-workspace-visual-rework.md) 已准备，限定为本 Story 原有视觉要求的缺陷修复；依赖已合入的真实 Workspace/Preview/Selection 和 F 编辑契约。本视觉切片进入 V1 实施：双列素材区、真实图片/视频缩略图、较大的场景缩略卡和面板去工程化已在 feature 分支提交，CI/审查和真实桌面视觉复验仍待完成；当前 main 视觉验收仍为 FAIL。必须用实际产品、丰富非私人媒体和并排截图复验，用户确认前不得关闭视觉缺陷。MA-D-01 仍独立待验，两个门槛都满足后才能关闭 Story。

## Delivery sequencing

The current implementation begins with a **Film Workspace visual baseline checkpoint** before the remaining playback/selection behavior. The approved Simplified-Chinese `docs/design/assets/v0.1/03-workspace.jpg` is the high-fidelity target for this checkpoint.

This sequencing change does not relax Story acceptance. The Story remains open until the behavioral criteria below are satisfied.

## Required real-desktop acceptance — MA-D-01 (pending)

**Human gate: NOT YET ACCEPTED.** The canonical `laputa-desktop` Film Workspace and automated Xvfb Scene Strip/selection checks are merged (PR #21; post-merge CI success). Real GUI user playback, seek, Selection, default edit target, and product-boundary confirmation are still required before Story D or Issue #5 can be closed.

Follow **[MA-D-01 checklist](../../acceptance/MANUAL-ACCEPTANCE.md#ma-d-01--story-d--issue-5--film-workspace-真实桌面交互)**. The checklist owns executable steps, evidence fields, and PASS/FAIL/BLOCKED recording; do not infer completion from CI or from the historical visual checkpoint (current visual review failed).

## Scope

- High-fidelity Film Workspace visual baseline matching the approved hierarchy, density and Nordic flat visual language.
- Deterministic preview render from timeline state.
- Play/pause/seek preview surface.
- Ordered selectable scene/clip strip.
- Current selection propagated as command context.

## Out of scope

- Default multi-track NLE, keyframe editor, effect inspector, professional scopes.

## Acceptance

- Given a valid first-cut timeline  
  When preview is requested  
  Then the user can play/seek the rendered result and see corresponding ordered scene-strip items.
- Given one scene-strip item is selected  
  When a later natural-language command is submitted without explicit scope  
  Then the application context marks that selection as the default edit target.

## Visual checkpoint gate

Before proceeding deeply into behavior, one production GPUI Film Workspace page should be reviewed against `03-workspace.jpg` and accepted as the visual tone baseline. It should prioritize Stage dominance, restrained side rails, a shallow Scene Strip, Simplified-Chinese copy, generous whitespace and large flat surfaces.

## Minimum validation

Visual baseline review + render fixture + GUI interaction smoke test; selection-context unit/integration tests.
