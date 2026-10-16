# Laputa Architecture Baseline

## System boundary

Laputa is an independent Rust desktop application. Talos is embedded as the generic Agent Loop runtime; Laputa owns all video-creation domain concepts and GUI behavior.

```text
GPUI Desktop
    |
    v
Laputa Project World
    |
    +--> Talos Runtime --> Laputa Tools
    |                       |\
    |                       | +--> Functional Providers --> Vendor/local adapters
    |                       +----> Deterministic engines (FFmpeg, timeline/render code)
    |
    +--> Asset/Artifact Store
    +--> Timeline IR
    +--> Preview/Export
```

## Responsibilities

### Talos

Talos owns generic provider interaction for the director model, tool calling, Agent loop, session/event flow, permission boundaries, cancellation/background execution, and reusable skill/runtime behavior. Laputa embeds Talos through `talos-runtime` and does not depend on Talos UI crates.

### Project World

Laputa persists the world the Agent observes and changes: `Project`, `IntentSpec`, `Asset`, `Shot`, `StoryPlan`, `Timeline`, `Clip`, `Artifact`, `GenerationJob`, `Critique`, and user locks/selections.

### Tool layer

Tools are coarse-grained Agent-understandable product actions such as `asset.analyze`, `timeline.edit`, `media.animate_photo`, `media.create_transition`, `render.preview`, and `video.critique`.

A Tool may call multiple providers and deterministic engines. A separate service layer is introduced only when orchestration must be reused by another entrypoint (for example both Agent tool and GUI command) or needs an independent lifecycle.

### Functional providers

Provider interfaces express technical capability, not vendor identity. V0.1 baseline interfaces are:

- `MediaUnderstandingProvider`
- `VideoGenerationProvider`
- `ImageGenerationProvider`
- `SpeechRecognitionProvider`
- `SpeechSynthesisProvider`
- `ArtifactStore`

`EmbeddingProvider` and specialized vision providers are deferred until a verified use case requires them.

Vendor adapters implement one or more functional interfaces. Normal users and Talos tools never choose vendor adapters directly.

### Deterministic media path

FFmpeg and native Rust media/timeline logic handle probing, trimming, composition, deterministic transitions/effects, audio mixing, preview and final encoding. Generative models are used only where semantic generation/rewriting is needed.

## Dependency direction

```text
laputa-app
  -> laputa-agent
  -> laputa-domain
  -> laputa-media
  -> laputa-timeline
  -> laputa-provider

laputa-agent -> talos-runtime

provider adapters (future crates) -> laputa-provider
```

Domain and provider contracts must not depend on vendor SDKs.

## Asynchronous generation contract

External generation follows `submit -> job -> terminal state -> artifact`. Submission success does not authorize timeline mutation. A generated asset becomes eligible for timeline use only after terminal success and artifact verification/materialization.

## Original-media contract

Imported original media is immutable. Trims, style changes, generated motion, transitions, proxies, thumbnails, audio derivatives and final renders are new artifacts or timeline operations referencing the original.
