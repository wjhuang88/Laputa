# Laputa Evolution Log

This file records reusable lessons learned from defects, corrections, regressions, planning drift, unexpectedly successful workarounds, and recurring Agent mistakes.

When a lesson is discovered, follow [docs/sop/EVOLUTION-FEEDBACK.md](docs/sop/EVOLUTION-FEEDBACK.md).

## Lesson index

| ID | Trigger | Prevention / promotion | Status |
| --- | --- | --- | --- |
| EVO-001 | Desktop documentation and binary argument parser drifted | Compare launch instructions with the real argument parser | Recorded |
| EVO-003 | Current product failed fidelity review despite historical completion | Require current real-product comparison and explicit user acceptance | Promoted to UI-UX vision |
| EVO-002 | Partial PR unexpectedly marked Story F complete | Avoid GitHub closing keywords in negative PR wording; verify issue state after merge | Recorded |

## EVO-001 — Verify real desktop CLI arguments before writing acceptance steps

- Trigger: F3a UI implementation review discovered README and Story D manual smoke suggesting `laputa-desktop create <path>` / `laputa-desktop open <path>`.
- Symptom: The current product binary's `initial_state` accepts only zero or one positional directory argument; the published command examples would fail before launching the correct project.
- Cause: Documentation retained aspirational subcommands after the desktop product entrypoint was consolidated; CLI examples were not checked against the parser.
- Remedy: Correct both READMEs to direct users to the launcher for creation and to pass one positional folder for opening; correct the MA-D-01 command. Preserve the actual binary interface without speculative flags.
- Prevention: Inspect the owning binary argument parser and test the documented command path before publishing manual acceptance instructions. When the CLI contract changes, update all real-user usage and acceptance files together.
- Promotion target: Documentation consistency review; no product CLI redesign implied.

## EVO-002 — Negative closing keywords can complete an unfinished Story

- Trigger: Partial F3a PR #34 was merged after exact-head CI success.
- Symptom: GitHub marked Issue #7 `Completed/Closed` even though F3b drag/language, F4 variants and all four acceptance criteria remained outstanding. GitHub issue events recorded the closure immediately after merge; the issue was reopened.
- Cause: The PR description used a negative sentence containing a GitHub issue-closing keyword next to `#7` (e.g. `do not ... close #7`). GitHub interpreted the keyword literally, not as a negation. This is the likely automation trigger, although the event payload does not contain an explicit parser explanation.
- Remedy: Reopen #7, preserve all unchecked criteria, synchronize parent Epic/Story and note actual PR #34 partial completion. Do not rely on prose negation to prevent automation.
- Prevention: For **partial** Stories, use `Issue #7 remains Open / In Progress`, not any GitHub closing keywords immediately followed by issue references. After each merged slice, explicitly verify associated issue and parent Epic state. Only full accepted delivery may use a deliberate closing keyword.
- Promotion target: PR authoring and post-merge issue verification practices; no unnecessary workflow redesign.

## EVO-003 — Historical visual acceptance does not prove current fidelity

- Trigger: 2026-11-16 user rejected the real desktop workspace as far from the approved mockup.
- Symptom: Status reporting called the visual baseline complete based on historical records before checking the current product; real screenshot showed placeholder cards and a different composition.
- Cause: Current reporting relied on historical acceptance and technical CI. Whether the original implementation or later changes caused each gap is unknown.
- Remedy: Record MA-D-VIS-01 FAIL with screenshot, keep Story D open and prepare bounded visual rework; preserve historical merge facts without treating them as current acceptance.
- Prevention: Use current real-product screenshots, representative media and matched viewport comparisons, then obtain explicit user acceptance. Static examples and automated smoke cannot establish high fidelity; record unsupported controls honestly.
- Promotion target: docs/design/UI-UX-VISION-v0.1.md visual acceptance evidence.
