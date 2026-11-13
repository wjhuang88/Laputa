# Laputa 人工验收台账（Manual Acceptance Ledger）

Status: maintained · Updated: 2026-11-04  
Scope: `LAPUTA-001` V0.1 · Owner of Story status: `docs/backlog/active/` and archived Story files

> **用途**：这是跨会话的人工验收入口。自动化 CI / Xvfb / fixture 测试不等于真实桌面用户确认；没有执行的操作必须保持未勾选。此文件记录待人工验证的事项和验收操作，不替代各 Story 的正式验收结论。

## 状态与边界

- **CURRENT / REQUIRED**：功能已合入、CI 通过，但 Story 仍然被某项真实用户验收阻塞。人工验证完成前不得关闭 Issue。
- **FUTURE / NOT READY**：需求中明确或合理预期需要 GUI / 最终产品实测，但功能仍在 Proposed 或未完成；不可以提前声称可操作，更不可以提前勾选。
- **OPTIONAL / CREDENTIAL-GATED**：需要用户自愿配置外部凭证/成本/素材的真实 Provider 试验；若 Story 已以确定性证据闭环，这类烟测不是追溯式的必需关闭门槛。
- **DONE / RECORDED**：历史验收已有证据；无需重新验收，除非新变更确实引入回归风险。

## 1. 当前需要你人工执行（唯一已知阻塞项）

### MA-D-01 · Story D / Issue #5 · Film Workspace 真实桌面交互

- **状态**：**CURRENT / REQUIRED · PENDING HUMAN CONFIRMATION**
- **Issue**：https://github.com/wjhuang88/Laputa/issues/5
- **Story**：`docs/backlog/active/LAPUTA-001-D-preview-scene-strip.md`
- **自动化已有证据**：PR #21 产品二进制在 Xvfb 中真实渲染、点击 Scene 02、校验 selection/default edit target；post-merge CI `40720360492` success。**这些不能替代下列真实桌面操作**。
- **准备**：使用真实 GUI 桌面（非 Xvfb/截图回放）；安装 Rust 1.95 与 FFmpeg（`ffmpeg`/`ffprobe`）；需要一个**已持久化 first-cut FilmState / Scene Strip 的项目**，至少包含两个 Scene。只有原始素材而没有 first cut 的空项目不满足本验收前提。
- **推荐：在本机生成可复现、无私人素材的两段测试项目**（在仓库根目录执行；目标目录**及同名 `.sources` 目录都必须尚不存在**，重复执行请换一个新目录名）：

  ```sh
  cargo run -p laputa-app --example workspace_fixture --locked -- /tmp/laputa-manual-d-01
  ```

  此命令使用项目已有的 `crates/laputa-app/examples/workspace_fixture.rs` 和 FFmpeg，生成 `京都静景`（图片 1.5s）+ `街头漫步`（视频 2s），并写入项目 `.laputa/project.json` 与 `.laputa/film.json`；来源同现有 `product-workspace-smoke.yml`，**不是把 CI 的 GUI 操作当作人工确认**。可使用你自己的已完成 first-cut 项目替代，但请避免在截图中泄露个人媒体。
- **在真实桌面启动产品**（直接打开上面创建的 fixture；如果用自己的项目，把路径替换掉）：

  ```sh
  cargo run -p laputa-app --features desktop-ui --bin laputa-desktop --locked -- /tmp/laputa-manual-d-01
  ```

  fixture 总时长约 3.5 秒，`±5s` 快捷 seek 可能钳制到片头/片尾；按**实际时间范围内画面是否同步**判定，不要求跳出短片范围。

- **逐项现场记录**（由用户实际操作后勾选；当前不得预填）：
  - [ ] **D-1 打开**：产品启动后加载该已有项目，显示真实影片 Stage、按顺序排列的 Scene Strip；无假数据或仅演示屏。
  - [ ] **D-2 播放**：点击 Play，Stage 画面实际随时间前进；Pause 能停止；画面与播放状态一致。
  - [ ] **D-3 Seek**：通过播放进度控制或现有 ±5s 控件前后跳转，画面与时间位置一致，非仅计时文本变化。
  - [ ] **D-4 Selection**：点击不同的 Scene Strip 条目，高亮项切换，Stage 按预期定位到该片段；不要求把“播放当前片段”误当作“编辑选中片段”。
  - [ ] **D-5 默认作用域**：选中 Scene 02 后，界面默认编辑目标显示“当前选中片段”或等价含义；切回/取消选择后，作用域遵循现有 Selection 规则。**此项只验证上下文，不要求 Story F 的实际语言修订**。
  - [ ] **D-6 异常恢复**：如果 Preview 失败，界面显示可见错误及重试路径；无失败时标记“未触发 / N/A”，不伪造故障。
  - [ ] **D-7 产品边界**：正常交互中无需选择 Provider、模型、Workflow；没有默认多轨专业剪辑控件。
- **通过标准**：D-1 至 D-5、D-7 均通过；D-6 对实际遇到的失败合理处理。任何无法执行项须记录 `BLOCKED` 与原因，不得用自动化证据代替。
- **需要的证据**：测试日期、本机 OS/桌面环境、被测 `main` commit SHA、项目是否含 FilmState/Scene 数、简短操作记录；建议提交播放前后与选中 Scene 02 后的截图/短录屏（私人媒体可脱敏或仅提供文字确认）、失败时错误文本；明确结论 `PASS / FAIL / BLOCKED`。
- **通过后**：把实际证据补充到 Issue #5、Story D 与对应 iteration；完成 Story D/Parent Epic/Backlog 的状态同步，满足其余验收后再关闭 #5。**本文件标记任务不构成验收通过**。

## 2. 后续产品功能实现后再人工执行（现在不要测试/勾选）

这些条目只是提前建立清晰验收入口，不表示实现已完成。开始测试之前先查看各 Story、对应 PR/main CI 与 GUI 功能是否实际可达。

| ID | Story / Issue | 状态 | 未来人工检查重点 | 自动化仍然必需 |
| --- | --- | --- | --- | --- |
| **MA-F-01** | F / #7 | **FUTURE / NOT READY** | 同一个 first cut：选择片段后说“这里短一点”，仅命中被选 Scene；显式“整部影片”覆盖选中范围；确认不是单纯上下文显示 | Scope-resolution / Talos Tool integration |
| **MA-F-02** | F / #7 | **FUTURE / NOT READY** | Scene Strip **拖动 B 到 A 前**，无需 Agent turn；直接 trim、delete；界面顺序/时长/删除结果持久化且源素材不被修改 | Deterministic timeline mutation / original-media hash |
| **MA-F-03** | F / #7 | **FUTURE / NOT READY** | Lock/retain 后全局自动修订不得修改锁定 Scene；显式解锁或覆盖才允许更新 | Lock guard 与 revision tests |
| **MA-F-04** | F / #7 | **FUTURE / NOT READY** | 生成派生媒体可“换一个 / Try another”，不暴露 Provider/模型参数；原版本/lineage 可追溯 | Variant/original immutability tests |
| **MA-G-01** | G / #8 | **FUTURE / NOT READY · PRODUCT WALKTHROUGH** | 展示 Critique 问题所对应 Scene/时间段；有界修复结束后仍存在的问题能在产品中被理解；不把主观观感当成替代 schema/budget 证据 | Critique schema、repair loop bounded termination 与 fixture e2e |
| **MA-H-01** | H / #9 | **FUTURE / NOT READY** | 编辑后**关闭应用再打开同一项目**：Intent、Scene 顺序、选择修改、锁定与派生 lineage 恢复；可以接着编辑 | Persist/reopen fixture 与 hash |
| **MA-H-02** | H / #9 | **FUTURE / NOT READY** | 产品内请求 16:9 与 9:16 导出；分别播放最终 MP4，检查正常画面、长度和尺寸；原始素材未改变 | ffprobe/playability/ratio/hash 自动验证 |
| **MA-MVP-01** | Epic / #1 | **FUTURE / NOT READY** | 一次真正的用户可达闭环：导入 → 一句话 → 可播放 first cut → 选中并修改 → 导出；全程无需 Provider/Model/Workflow 选择 | 跨模块集成 fixture 与 original hash |

### F3a 部分产品预检 — Story F / Issue #7（不得替代 MA-F-02/03）

- **状态：F3a PR #34 合并且最新 main CI 通过后才可执行的部分人工预检；UNRUN。** 这是完成 Story F 前可独立执行的一组验证，不代表拖动/自然语言编辑/生成变体已经实现。
- **入口：** 在真实桌面上执行以下命令；两个目标目录都需不存在，重复操作请换新名称。原始媒体是 FFmpeg 生成的非私人 fixture。

  ```sh
  cargo run -p laputa-app --example workspace_fixture --locked -- /tmp/laputa-manual-f3a-01
  cargo run -p laputa-app --features desktop-ui --bin laputa-desktop --locked -- /tmp/laputa-manual-f3a-01
  ```

- **预检 1 — 直接排序：** 底部选择“街头漫步”Scene 02，在右侧点击“← 前移”；核对 Scene Strip 立即变为 Scene 02 → Scene 01，Stage 真实预览内容及片段顺序更新。然后点击“后移 →”恢复顺序；全程不要求 Agent 回合。
- **预检 2 — 缩短/持久化：** 保持选中“街头漫步”，点击“缩短 20%”；片段时长应从 2.0s 变为 1.6s，影片总时长从 3.5s 变为约 3.1s。观察预览重新生成后画面/进度同步；关闭再用同一个项目路径打开，顺序和时长保持一致。
- **预检 3 — 保留/错误保护：** 点击“保留这个片段”；再尝试对这个片段“缩短 20%”或“删除片段”，应显示可理解的错误且不改影片。点击“取消保留”后删除 Scene 02；应只剩 Scene 01，Stage 更新且原始素材仍存在。最后一个 Scene 的删除必须被拒绝。
- **预检 4 — 异步旧帧：** 可在预览生成中快速连续操作，核对上一次预览结束后不会恢复旧顺序/旧时间；如果无法稳定复现就标记 N/A，不伪造竞态证据。
- **仍未完成：** F3b 鼠标拖放、精细裁切手柄、真正自然语言提交和 F4 “换一个”。MA-F-01～04 保持原状态，尤其**按钮前移不等于 MA-F-02 的拖动成功**。
- **实际执行后记录：** 日期、OS/桌面、被测 `main` commit、操作结果 PASS/FAIL/BLOCKED（每步可分别记录）、截图/短录屏及异常文本。未经本人真实操作不勾选或宣布通过。

- Story F：`docs/backlog/active/LAPUTA-001-F-selection-revision.md`。
- Story G：`docs/backlog/active/LAPUTA-001-G-critic-repair.md`。其已写明的强制 gate 以结构化/有界 fixture 证据为准；MA-G-01 只作为**产品可用性走查**，在 Owner 正式把它列为验收 gate 前，不强加新的人工阻塞条件。
- Story H：`docs/backlog/active/LAPUTA-001-H-persistence-export.md`。
- Epic：`docs/backlog/active/LAPUTA-001-agentic-vlog-mvp.md`。
- Story F/G/H 的人工检查依赖届时的真实产品接线；不以目前的 D UI 或 E provider 基础能力替代未来行为。

## 3. 外部 Provider 的可选真实 smoke（不要与人工关闭门槛混淆）

| ID | Story | 状态 | 执行前提 / 既有事实 |
| --- | --- | --- | --- |
| **MA-EXT-B** | B / #3 · Gemini understanding | **OPTIONAL / CREDENTIAL-GATED · UNRUN** | `crates/laputa-provider-gemini/tests/live_smoke.rs`；需要用户自愿提供 `GEMINI_API_KEY` 和适当素材。确定性验证已支持 Story B 完成。 |
| **MA-EXT-C** | C / #4 · Talos first-cut | **OPTIONAL / CREDENTIAL-GATED · UNRUN** | 需要可工作的真实 Talos LLM provider/fixture；历史记录只有确定性 fake provider 证据，不要声称已跑过 live 测试或捏造命令。 |
| **MA-EXT-E** | E / #6 · Runway generation/materialization | **OPTIONAL / CREDENTIAL-GATED · UNRUN** | `crates/laputa-provider-runway/tests/live_smoke.rs`；需要用户自愿配置 `RUNWAYML_API_SECRET`、可用 API 配额。E5b 已通过无网络的真实 Tool→E3 组合验证，Story E 已关闭。 |

**安全与费用**：绝不把 Token/API Key 或私人媒体内容提交 GitHub/对话/日志；可从本机环境注入；开始前确认第三方计费与隐私。未授权不运行付费 API。未运行保持 `UNRUN`，不用 `PASS`、也不回滚已完成 Story 的闭环。

## 4. 历史已完成的人工验收（勿重复列入待办）

- **MA-A-01 · Story A / #2 — DONE / RECORDED**：2026-10-20 用户在真实桌面完成素材导入 → 关闭/重启 → 重开同一项目 → Asset Browser 素材清单恢复，并确认“符合预期”。详见 `docs/backlog/archive/LAPUTA-001-A-project-media-import.md` 与 `docs/iterations/2026-10-17-gpui-project-import.md`。除真实回归问题外无需重新验收。
- **Story D 视觉 checkpoint — DONE / RECORDED**：PR #17 的视觉基线及迭代 `2026-10-23-workspace-visual-baseline.md` 已接受；**不等同于** MA-D-01 真机交互验收。

## 5. 另一个会话的接力入口

在新会话中直接粘贴：

> 请接手 `wjhuang88/Laputa` 的人工验收引导。首先读取 `docs/acceptance/MANUAL-ACCEPTANCE.md` 和对应 Issue/Story。优先执行 **MA-D-01 / Issue #5**，逐步指导我在真实桌面操作，每一步问我实际结果并记录 PASS/FAIL/BLOCKED。不得把自动化 Xvfb/CI 当成人工确认；不得擅自把 FUTURE 或 OPTIONAL 项勾选为通过。验收后通过 GitHub 更新实际证据与状态。保留已完成 Story A/E 的历史事实。

## 6. 台账维护规则

1. 新增任何**必须人工介入**的验收条件时，在同一个交付 PR 中登记 ID、状态、步骤、通过标准、证据和 Issue 链接；各 Story 链接此台账，不重复维护多份独立清单。
2. 完成实际人工测试后才从 `PENDING` → `DONE`；写明执行者确认/日期/环境/commit、结果和证据位置。没有用户反馈时不能推定 PASS。
3. CI 通过只能更新自动化证据，不能修改人工确认结论；阻塞、付费凭证或真实桌面访问缺失要明示。
4. 新会话使用此文档及 GitHub 最新 Issue/PR/main 作事实源，不使用过期 README 或聊天摘要代替仓库状态。
