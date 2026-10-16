# Laputa Product Vision

## Product promise

Laputa helps ordinary people turn personal photos and videos into a coherent finished video without learning professional editing concepts or AI workflow tools.

The primary mental model is deliberately small:

> Add media → say what you want → watch the result → select what is wrong and say how to change it → export.

## Target user

A consumer who has personal photos/videos and wants a shareable Vlog, memory film, travel video, wedding/family recap, or short personal story, but does not want to learn tracks, keyframes, model parameters, workflow graphs, or provider choices.

## Interaction principles

1. **Intent over mechanism.** The user describes the desired outcome; Laputa chooses editing/generation techniques.
2. **Selection is context.** When content is selected, natural-language edits target that selection by default. Without selection, instructions target the current video/project. Explicit language can override scope.
3. **Direct manipulation for spatial operations.** Ordering, trimming, deletion, playback/seek, selection, and lock/unlock should be direct GUI operations where that is clearer than language.
4. **Few choices.** Normal creation must not require selecting a model, provider, generation workflow, transition algorithm, timeline track, or tuning parameter.
5. **Safe defaults and undoability.** The system should act instead of repeatedly asking routine questions when the action is non-destructive and reversible.
6. **Trust through stability.** Users can mark a satisfactory part as retained/locked so later automatic revisions avoid changing it.

## Product boundary

Laputa is not a simplified Premiere/DaVinci clone and is not a ComfyUI-style workflow builder. A professional multi-track editor may become an advanced surface later, but it is not the primary product model.
