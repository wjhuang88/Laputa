# Git Workflow

Laputa begins with a simple branch model: short-lived feature/fix branches when useful, normal pull-request review for shared work, and no mandatory GitFlow until scale/release evidence requires it.

Before changes: inspect branch/worktree and `git status --short`; preserve unrelated work. Before commit: stage only the active Story slice and inspect staged diff/stat.

Agent commit format is mandatory:

```text
type(scope): description (#story-id) [model:<model-name>]
```

Use the true active model name for Agent-authored/assisted commits. Commit completed checkpoints before beginning a new staged development phase. Do not force-push or rewrite shared history without explicit approval.

If parallel Agents/worktrees or release branches become common, reassess branch/worktree policy from actual repository scale rather than adopting GitFlow pre-emptively.
