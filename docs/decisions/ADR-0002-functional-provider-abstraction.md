# ADR-0002 — Abstract providers by functional capability

## Context

V0.1 will use external AI APIs to avoid blocking product validation on model deployment. Different vendors may provide understanding, image/video generation, ASR or TTS; later versions should be able to substitute local/open models.

## Constraint Decomposition

| Constraint | Type | Source | Can Change? |
| --- | --- | --- | --- |
| V0.1 may rely on external AI services | Soft | Delivery strategy | Yes |
| Core product logic must survive vendor replacement | Hard | Long-term architecture goal | No |
| A single vendor may implement several capabilities | Hard | External API reality | No |
| Tool behavior may need multiple providers/engines | Hard | Product operations such as animate/transition | No |
| Provider routing sophistication is needed immediately | Assumption | Unvalidated future need | Maybe |

## Reasoning

Vendor-first abstractions (`MiniMaxProvider`, `RunwayProvider`) make product logic inherit vendor boundaries and complicate future local replacement. Over-splitting every model operation into tiny interfaces creates equal complexity. A small set of functional contracts plus capability metadata provides a stable middle boundary.

## Decision

Core interfaces are functional (`MediaUnderstandingProvider`, `VideoGenerationProvider`, `ImageGenerationProvider`, `SpeechRecognitionProvider`, `SpeechSynthesisProvider`, `ArtifactStore`). Vendor/model adapters implement these interfaces and may share an internal vendor HTTP client.

Talos tools express domain actions and may directly orchestrate multiple providers and deterministic engines. A separate domain-service layer is optional and appears only when orchestration is reused by multiple entrypoints or has independent lifecycle/state.

Normal users and normal Tool schemas never select vendors/models directly.

## Reversal Trigger

Revisit if real implementations prove that a functional interface combines incompatible lifecycle/error semantics, or if repeated cross-tool orchestration requires a stable reusable domain service. Split from evidence, not pre-emptively.
