# Laputa Evolution Log

This file records reusable lessons learned from defects, corrections, regressions, planning drift, unexpectedly successful workarounds, and recurring Agent mistakes.

When a lesson is discovered, follow [docs/sop/EVOLUTION-FEEDBACK.md](docs/sop/EVOLUTION-FEEDBACK.md).

## Lesson index

| ID | Trigger | Prevention / promotion | Status |
| --- | --- | --- | --- |
| EVO-001 | Desktop documentation and binary argument parser drifted | Compare launch instructions with the real argument parser | Recorded |

## EVO-001 — Verify real desktop CLI arguments before writing acceptance steps

- Trigger: F3a UI implementation review discovered README and Story D manual smoke suggesting `laputa-desktop create <path>` / `laputa-desktop open <path>`.
- Symptom: The current product binary's `initial_state` accepts only zero or one positional directory argument; the published command examples would fail before launching the correct project.
- Cause: Documentation retained aspirational subcommands after the desktop product entrypoint was consolidated; CLI examples were not checked against the parser.
- Remedy: Correct both READMEs to direct users to the launcher for creation and to pass one positional folder for opening; correct the MA-D-01 command. Preserve the actual binary interface without speculative flags.
- Prevention: Inspect the owning binary argument parser and test the documented command path before publishing manual acceptance instructions. When the CLI contract changes, update all real-user usage and acceptance files together.
- Promotion target: Documentation consistency review; no product CLI redesign implied.
