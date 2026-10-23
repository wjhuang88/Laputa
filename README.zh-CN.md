# Laputa

[English](README.md) | [简体中文](README.zh-CN.md)

Laputa 是一个面向普通用户的智能桌面视频创作产品。它希望把个人照片和视频，通过一个足够简单、没有歧义的交互闭环，转化为完整的 Vlog 成片：

> 添加素材 → 描述想要的结果 → 查看第一版 → 选中并修改 → 导出。

Laputa 使用 [Talos](https://github.com/wjhuang88/talos) 作为内嵌的 Agent Loop Runtime。产品领域模型、项目状态、媒体工具、Timeline、Provider 抽象、渲染能力和 GUI 均由 Laputa 自己负责。

## 产品目标

Laputa 不希望把传统专业剪辑软件简单“套一层 AI”，也不希望把 ComfyUI 一类工作流节点直接暴露给普通用户。

核心原则是：

> 用户表达“想要什么”，系统负责决定“怎么做”。

普通用户不需要理解模型、Provider、工作流、Track、关键帧、生成参数等技术概念。默认操作模型尽量收敛为：

- **说**：用自然语言描述创作目标或修改意图。
- **选**：选中某个场景、片段或素材，作为默认修改作用域。
- **拖**：在顺序、时长等空间关系上使用最直觉的直接操作。

## 当前状态

产品治理、架构、V0.1 需求与 UI/UX 视觉基线已经建立。当前 `LAPUTA-001-A` 处于 **In Progress**：项目持久化、非破坏性混合媒体导入、媒体探测边界、自动化安全测试，以及 GPUI 项目创建/打开、系统文件选择导入和 Asset Browser 展示路径已经实现。

Story A 目前唯一没有证据闭环的必需项是：在真实图形桌面上完成一次 **导入 → 关闭/重启 → 打开同一项目 → 素材清单仍可见** 的人工 smoke。因此不会仅因为 CI 能编译桌面程序就提前把 Story 标记为 Complete。

### 桌面 GUI

桌面产品入口收敛为唯一显式声明的 `laputa-desktop` binary，并通过 `desktop-ui` feature 启用。

启动项目首页：

```sh
cargo run -p laputa-app --features desktop-ui --bin laputa-desktop
```

直接创建项目：

```sh
cargo run -p laputa-app --features desktop-ui --bin laputa-desktop -- \
  create ./demo-project "我的 Vlog"
```

直接打开已有项目：

```sh
cargo run -p laputa-app --features desktop-ui --bin laputa-desktop -- \
  open ./demo-project
```

桌面导入使用系统多文件选择器，媒体 probe 继续复用 `ffprobe`，因此需要安装 FFmpeg。导入工作在 UI 线程之外运行；成功后 Asset Browser 会直接从最新 `ProjectManifest` 展示素材，失败则保留已有项目状态并显示可恢复错误。Laputa 只读引用导入的原始素材；项目元数据、缓存以及后续派生文件均保存在 `<project>/.laputa/` 下，不覆盖原文件。

当前 Asset Browser 先展示媒体类型、文件名以及可用的时长/尺寸元数据，不以缩略图生成扩大 Story A 范围。视觉层级遵循 [Laputa V0.1 UI/UX 基线](docs/design/UI-UX-VISION-v0.1.md)。

当前尚未进入的能力包括素材 AI 理解、Talos First Cut、生成式媒体、可播放 Preview、真实 Scene Strip 编辑、Critic 和最终导出交互，它们仍按 `LAPUTA-001` 的 A→H 任务链渐进实现。

## 核心产品原则

- **面向大众**：默认流程中不要求用户理解专业剪辑和 AI 基础设施概念。
- **Agent 驱动**：Talos 根据用户创作意图，自主调用粗粒度 Vlog Tools 完成任务。
- **直接操作优先于对话的场景仍保留直接操作**：选择、拖动排序、裁切、删除、播放、锁定等动作无需强制经过 Agent。
- **Selection 即上下文**：有明确选中对象时，自然语言默认作用于 Selection；没有 Selection 时默认作用于当前成片；显式语言作用域可以覆盖 Selection。
- **Provider 按功能抽象**：稳定边界是媒体理解、视频生成、图片生成、语音识别、语音合成等能力，而不是 MiniMax、Gemini、Runway 等供应商名称。
- **Tool 可以自然编排多个 Provider**：Tool 表达 Agent 能理解的业务动作，内部可以组合多个 Provider 和确定性媒体引擎，不强制增加无意义的 Service 层。
- **非破坏性编辑**：原始素材不可修改，所有编辑和生成内容都作为独立派生 Artifact 存在。
- **外部 API 可作为 V0.1 启动方案**：第一版优先验证产品闭环，而不是先投入本地模型部署；后续可以在 Provider 边界下逐步替换为本地或开源实现。

## V0.1 用户主流程

V0.1 重点验证一条单一、直觉的主路径：

1. 用户创建项目并导入照片/视频。
2. Laputa 自动理解素材并形成可复用的结构化 Project World。
3. 用户用一句自然语言描述目标，例如：
   > “帮我做一个 60 秒左右的京都旅行 Vlog，自然温暖一点，重点突出两个人。”
4. Talos 自动选择素材、组织故事并生成 First Cut。
5. 用户直接观看 Preview，并通过 Scene Strip 理解当前视频结构。
6. 用户选中某个片段后说“这里短一点”“换一个”“人物别动太多”，系统自动把 Selection 作为默认修改范围。
7. 用户也可以直接拖动排序、裁切、删除或锁定满意片段。
8. Laputa 可对 Preview 做结构化 Critique，并在明确预算/重试边界内自动修复明显问题。
9. 用户导出最终 MP4。

## 架构边界

```text
                     Laputa Desktop App
                            │
                            ▼
                       Project World
                            │
                            ▼
                       Talos Runtime
                            │
                         Vlog Tools
                            │
              ┌─────────────┼─────────────┐
              ▼             ▼             ▼
          Providers      Local Media    Timeline
                         Engines         / Render
              │             │
              ▼             ▼
       外部 API / 本地模型   FFmpeg 等
```

### Talos 负责

- Agent Loop
- Tool Calling
- Session / Context
- Skills
- Provider-independent Agent execution
- Background job / runtime event 能力

### Laputa 负责

- Vlog / Project 领域模型
- Asset / Story / Timeline / Artifact
- Vlog Tools
- 功能型 Provider 抽象
- Provider 路由和降级
- FFmpeg / 媒体处理
- GUI
- Preview / Export

Talos 不应感知 `Shot`、`Timeline`、Vlog、FFmpeg 或具体生成模型等 Laputa 领域概念。

## Workspace 结构

- `crates/laputa-app`：桌面应用与 GPUI 产品入口。
- `crates/laputa-domain`：Project World 与领域模型。
- `crates/laputa-agent`：Talos Runtime 组合与 Laputa Tools。
- `crates/laputa-provider`：按功能定义的 Provider Contracts。
- `crates/laputa-media`：确定性媒体处理、项目持久化、非破坏性导入与后续 Artifact 管理。
- `crates/laputa-timeline`：Timeline IR 与渲染侧编辑模型。

## 需求与开发入口

- [V0.1 产品需求](docs/reference/PRODUCT-REQUIREMENTS-v0.1.md)
- [UI/UX 视觉基线](docs/design/UI-UX-VISION-v0.1.md)
- [架构基线](docs/reference/ARCHITECTURE.md)
- [产品愿景](docs/reference/PRODUCT-VISION.md)
- [LAPUTA-001 MVP Epic](docs/backlog/active/LAPUTA-001-agentic-vlog-mvp.md)
- [GitHub Issues](https://github.com/wjhuang88/Laputa/issues)
- [项目治理与文档地图](docs/README.md)
- [Agent 开发规则](AGENTS.md)

## 当前 MVP 任务链

- #1 `LAPUTA-001`：Agentic Consumer Vlog MVP
- #2 `LAPUTA-001-A`：项目创建与非破坏性混合媒体导入（In Progress；基础与 GPUI 路径已实现，待人工桌面 smoke）
- #3 `LAPUTA-001-B`：素材结构化理解与功能型 Provider
- #4 `LAPUTA-001-C`：Talos 驱动的一句话 First Cut
- #5 `LAPUTA-001-D`：Preview + Scene Strip + Selection 上下文
- #6 `LAPUTA-001-E`：生成式派生媒体与异步 Job
- #7 `LAPUTA-001-F`：Selection 作用域修改与直接操作
- #8 `LAPUTA-001-G`：Preview Critic 与有界自动修复
- #9 `LAPUTA-001-H`：工程恢复与最终 MP4 导出
