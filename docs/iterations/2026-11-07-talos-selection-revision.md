# Iteration — Talos Selection Revision Tool (F2)

Status: partial — F2 Tool/Runner implementation and tests submitted in PR #32; Story F remains open  
Date: 2026-11-07  
Selected Story: `LAPUTA-001-F` / Issue #7 (F2 only)  
Baseline: `main` `ca3609034287d51801e4d624227be317ceb6efb3` (PR #31 F1 merged; post-merge CI 42183222768 success)

## Goal and dependency decision

Wire the existing canonical F1 `FilmState::revised` and `FilmStore` into **real Talos Runtime + Laputa Tools**, so a natural-language request with existing Selection context can stage and safely commit one deterministic scene-edit operation.

Story D / #5 remains open for MA-D-01 real-desktop interaction. F2 is a **bounded, integration-testable technical exception**, not an assertion that D's manual check passed; Story F remains open for F3 GUI and F4 variant + real manual MA-F-01…04.

## Success and scope (fixed before implementation)

1. `timeline.get` returns a read-only canonical scene/locked/duration snapshot and explicit Selection context, with no original-media paths, vendor or model details.
2. Talos `timeline.edit` accepts a **single coarse product action**, initially `shorten` or `delete`. The runtime supplies actual Selection; the model cannot invent a scene ID to redirect a default edit.
3. A selected Scene is the default target; no selection defaults to Project. A selected Scene is overridden by whole-video scope **only if the user's original request explicitly says whole/entire video**. Unsupported or ambiguous scope must fail closed, not silently widen editing.
4. `shorten` deterministically reduces displayed duration by 20% (an initial **product assumption**, not a user-facing knob); `delete` requires selected scene scope and never deletes the last scene. Project shortening applies to unlocked scenes while preserving locked scenes. A future UX evaluation can adjust the default.
5. Tools **stage** one validated candidate FilmState only; the runner commits once **after a successful Talos turn**, only if the project/film baseline remains unchanged. Malformed/unsupported calls, stale selections, locks, cancellation, failed turns or no edit never claim success or write a new film.
6. Deterministic scripted Talos provider tests prove selected shortening, explicit whole override, blocked model-invented scope, locked Scene immunity, unsupported semantic requests, no-commit on failure and original-media immutability. Do not use real paid model credentials.

## Deliberate exclusions

- No GPUI command-input wiring, timeline refresh, visible direct reorder/trim/delete/lock controls (F3).
- No model-directed project-wide arbitrary reshaping or color grading/stylization claimed as rendered effects; no generated “Try another” (F4).
- No Talos runtime generic APIs modified; all domain behavior stays in `laputa-agent` and `laputa-timeline`.
- No guaranteed general-language intent coverage: F2 uses a constrained operation gate. Requests outside its supported subset fail transparently; validate real UX in MA-F-01 after F3.
- No concurrent multi-writer support yet. Runner checks a stale baseline before commit; single-writer is currently assumed and future concurrent needs require explicit fencing.

## Validation

- `cargo fmt --all -- --check`, `cargo test -p laputa-agent`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, full repository CI and FFmpeg fixture.
- Evidence requires exact PR head CI, PR review and merge status; final post-merge main CI checked separately.
- Manual MA-D-01 and MA-F-01…04 remain **UNRUN**, F2 alone is not Story F closure.

## Implemented checkpoint / observed CI (2026-11-07)

- **[PR #32](https://github.com/wjhuang88/Laputa/pull/32)** implements `TimelineGetTool`, `TimelineEditTool`, `RevisionRunner` and publishes the modules under `laputa-agent`. No changes were made to Talos Runtime, original media, provider contracts or default GPUI behavior.
- A real `RuntimeBuilder` running against a deterministic scripted `LanguageModel` exercises Tool inspection, scoped edit staging and a one-time FilmStore write after success. Negative cases verify that malformed/unsupported edits, invented global scope, an incomplete turn, repeated Tool calls and uninspected editing cannot leave partial state. No model network calls or private credentials were used.
- First CI head `d7a5bc9b0e1f545eccb350b8eda2c6da32f39264` failed at `cargo fmt --all -- --check` (after FFmpeg fixture success); exact Rustfmt diffs were applied in the two new Rust files. This is **not** a behavioral test failure.
- Corrected code head `e01dcadb271cd74eb6dbe61c744adcead2c88515` passed exact-head [push CI #42341440961](https://github.com/wjhuang88/Laputa/actions/runs/42341440961) **success** and [PR CI #42341443001](https://github.com/wjhuang88/Laputa/actions/runs/42341443001) **success**.
- Follow-up regression tests explicitly cover no Selection → Project default and locked selected Scene rejection. **Do not substitute the preceding CI results for the newest head**; later tests/documentation commits require their own push/PR CI before merge.
- `timeline.edit` in F2 intentionally supports only shorten and delete selected; 20% shortening is an unvalidated UX default, color/warmth instructions are unsupported, and a literal scope keyword guard is conservative rather than general natural-language understanding.
- **No product GUI human acceptance was performed.** MA-D-01 remains CURRENT/REQUIRED; MA-F-01…04 remain FUTURE/NOT READY. F3 command/preview wiring, F3 direct editing, F4 generated variants and Story F acceptance remain outstanding.

## Disposition / next steps

**Partial**: This is a bounded technical integration checkpoint, not full Story F and not yet a merged/accepted PR at the time of this evidence entry. After final exact-head PR/push CI pass, review/merge PR #32 with expected head and inspect post-merge main CI. Record final merge/evidence in Issue #7; keep all user-facing acceptance checkboxes unchecked until actual F3/F4 implementation and human verification.

