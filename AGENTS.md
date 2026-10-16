# Laputa Agent Rules

Laputa is a consumer desktop application that uses Talos as its embedded Agent Loop runtime. Read this file before changing code, requirements, governance, architecture, or delivery state.

## Hard Constraints

- Preserve original user media. No tool, provider, renderer, test, or cleanup path may overwrite or destructively mutate imported originals.
- Talos is an embedded runtime dependency, not the owner of Laputa domain concepts. Do not add Laputa-specific `Asset`, `Shot`, `Timeline`, Vlog, provider-routing, or GUI behavior to Talos merely to simplify this application.
- The model-facing interface is coarse-grained Laputa tools. Do not expose vendor APIs, FFmpeg command details, sampler/model parameters, or workflow-node graphs as normal user-facing or Agent-facing product controls when a domain tool can own the orchestration.
- Functional provider contracts are the stable technical boundary. Vendor/model adapters implement those contracts; vendor names must not define core domain interfaces.
- A Tool may naturally orchestrate multiple providers and deterministic engines. Add a separate service layer only when logic is reused by multiple entrypoints or has an independently useful lifecycle; do not add pass-through abstraction layers.
- User-visible editing must remain deterministic in scope: an explicit selection is the default target; without a selection the current video/project is the target; explicit natural-language scope overrides selection.
- Do not start implementation work from an Epic alone. Select a Ready child Story whose dependencies and acceptance evidence are explicit.
- Route requirement changes through `docs/sop/REQUIREMENT-INTAKE.md`; route changes to in-progress accepted scope through `docs/sop/CHANGE-CONTROL.md`.
- Original requirements, decisions, and accepted architecture boundaries must be updated in their owning documents rather than kept only in chat, commit messages, or code comments.
- Before process/governance work, use the Task Router below and read the owning document first.

## Coding Behavior

- State material assumptions before implementing them. If an assumption affects architecture, privacy, provider capability, user behavior, or acceptance, record a validation path or Spike rather than silently treating it as fact.
- Implement only the requested Story/scope. Do not add speculative features, premature abstractions, unsolicited refactors, or generic frameworks because they might be useful later.
- Match the existing repository style and boundaries even when another design is personally preferable. Architecture changes require the appropriate decision/change-control record.
- Define verifiable success criteria before editing production code. Behavior-facing work requires observable acceptance; technical work requires commands or concrete manual evidence.
- Clean up only files, code, configuration, or artifacts orphaned by your own change. Do not use a scoped Story as permission for unrelated cleanup.
- Keep user-visible behavior simple. Do not introduce optional branches, provider/model selectors, tuning knobs, or professional-editor concepts unless a requirement explicitly demonstrates that the simpler interaction is insufficient.
- Treat external AI output as untrusted input: validate structured responses, handle timeout/cancellation/failure, and never infer successful artifact creation from a submitted remote job alone.

## Git Rules

- Inspect `git status --short` before work and before commit. Do not overwrite unrelated user or Agent changes.
- Stage only files belonging to the current Story or explicitly approved slice. Inspect the staged diff/stat before committing.
- Commit checkpoints before starting a new staged development phase when the current phase is complete.
- Use Conventional Commits with the format: `type(scope): description (#story-id) [model:<model-name>]`.
- `scope` should name the affected Laputa area (`app`, `agent`, `domain`, `provider`, `media`, `timeline`, `docs`, `governance`, or `workspace`).
- `(#story-id)` uses the active work identity when one exists, for example `(#LAPUTA-001-A)`.
- `[model:<model-name>]` is required for every Agent-authored or Agent-assisted commit. Use the true active model name at commit time; never copy the tag from an older commit.
- Never force-push, rewrite shared history, or commit generated secrets/API credentials unless the user explicitly requests and reviews that action.
- See `docs/sop/GIT-WORKFLOW.md` before branch, worktree, merge, release, or multi-Agent Git operations.

## Task Router

| Task | Read first |
| --- | --- |
| Understand product intent or current architecture | `docs/reference/PRODUCT-VISION.md`, `docs/reference/ARCHITECTURE.md` |
| Understand V0.1 requirements | `docs/reference/PRODUCT-REQUIREMENTS-v0.1.md` |
| Add/refine requirements or split Epic/Story | `docs/sop/REQUIREMENT-INTAKE.md`, `docs/backlog/PRODUCT-BACKLOG.md` |
| Start an implementation batch | `docs/sop/START-ITERATION.md`, `docs/sop/ITERATION-WORKFLOW.md` |
| Change accepted/in-progress scope | `docs/sop/CHANGE-CONTROL.md` |
| Add product behavior | `docs/sop/NEW-FEATURE.md` and the selected Story |
| Run or debug locally | `docs/sop/LOCAL-DEV.md` |
| Test or validate | `docs/sop/TESTING.md` |
| Git/branch/worktree/commit | `docs/sop/GIT-WORKFLOW.md` |
| Long multi-phase work | `docs/sop/LONG-RUNNING-TASK.md` |
| Update or audit docs | `docs/sop/DOC-CHECK.md`, `docs/README.md` |
| Record or review architecture choices | `docs/decisions/README.md` |
| Record lessons after defects/corrections | `docs/sop/EVOLUTION-FEEDBACK.md`, `EVOLUTION.md` |
| Governance repair/upgrade | compare local governance against the current `agent-project-governance` skill before editing |

## Current Known Traps

- Talos source is pinned intentionally. Do not silently move to Talos `main`; update the pin through an explicit dependency change and re-run integration validation.
- A submitted external generation job is not a generated artifact. Persist job state and verify the completed artifact before modifying timeline state.
- Do not equate `animate_photo` with image-to-video. The Tool may choose deterministic motion, generated video, or no motion based on context and constraints.
- Do not expose provider/model selection to normal users as a workaround for routing quality. Fix routing/fallback logic below the consumer interaction layer.
- A traditional multi-track NLE is intentionally out of V0.1 default UI. Do not introduce it without a requirement/change-control record.

## Session End Checklist

Before declaring a Story, iteration, governance slice, or change complete:

- Re-read acceptance criteria and record the actual verification evidence; a file existing is not evidence of behavior.
- Synchronize the owning Story/Epic and any active iteration record before summarizing status.
- Classify important conclusions as confirmed fact, inference, assumption, or unknown; register a validation path for anything that affects completion.
- Register residual work in the backlog or owning record; do not leave required follow-up only in chat.
- If a repeated failure, corrected omission, newly discovered trap, or important workaround occurred, update `EVOLUTION.md` through `docs/sop/EVOLUTION-FEEDBACK.md`.
- Ask: did this session make a technical choice that affects a Soft or Assumption constraint? If yes, record/update an ADR in `docs/decisions/`.
- Ask: did this session change observable behavior? If yes, update the corresponding user-facing README/usage documentation in the same delivery slice.
- If `docs/BOARD.md` is later introduced, update owner documents first and the derived board second.
- Inspect `git status --short` and the staged diff before any commit. Ensure the commit format includes the required true model tag.
- Report closure strictly as `complete`, `partial`, or `blocked`, including verification and residuals.
