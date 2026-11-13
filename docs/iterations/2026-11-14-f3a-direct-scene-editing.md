# Iteration — F3a Direct Scene Editing and Preview Refresh

Status: partial — F3a implementation merged with CI; real desktop preflight and Story F completion still pending  
Date: 2026-11-14  
Selected: `LAPUTA-001-F` / Issue #7 (F3a)  
Baseline: `main` `a886020a11be98729bb35e5c296f7ef6dd9f5edc` (F1 PR #31 + F2 PR #32 + docs #33 merged; main CI #42344231989 success)

## Goal

Expose the merged F1 canonical timeline revision core in the **real GPUI `laputa-desktop` Film Workspace**, giving users immediate, source-preserving scene actions and correctly invalidating/rebuilding preview after each successful edit. No provider credential or Talos turn is required for direct manipulation.

## Readiness/dependency

Story D Film Workspace/Selection/Preview production code and automated Xvfb smoke are merged; its real desktop MA-D-01 gate remains pending. F3a uses those existing merged contracts only and is a bounded independently testable product slice; it does not claim D or Story F has passed real-desktop user acceptance.

## Fixed acceptance for F3a

1. A selected scene exposes discrete, direct actions to **move earlier/later**, **shorten by a clearly labeled 20%**, **remove**, **lock/unlock**. Moving must preserve Scene ID and media source; operations on a locked target cannot silently mutate it. Lock toggle is explicit user action, not an Agent turn.
2. All operations reuse `FilmState::revised` / `FilmStore::apply_revision`; on error, original `film.json` and GUI projection remain stable. Deleting the last Scene is rejected; locked scenes are protected.
3. After successful persistence, rebuild the `WorkspaceState`, retain valid Selection (or clear deleted selection), pause playback, clamp seek, invalidate existing cached playback/frame and re-render preview using the new timeline.
4. Async preview render results started before the revision must be ignored; a stale render may not overwrite the new preview. No source original is changed or deleted.
5. Headless Rust tests cover scene move/shorten/delete/lock, reopened persisted state, Scene Strip order and preview plan, failure rollback, and unaffected original bytes. Normal desktop binary CI compile/smoke proves GPUI UI wiring; real-desktop human MA-F verification remains pending.

## Explicit exclusions / follow-on

- F3b: true pointer drag-and-drop reorder, a real text editing / submit control wired to Talos `RevisionRunner` with configured provider, advanced trim range interaction. This slice provides discrete arrow moves, not drag acceptance.
- F4: generated artifact variant “Try another”; no video color grading claims.
- No multi-track editor, external paid provider, Talos core change, source media mutation, or structural schema redesign.

## Validation

`cargo fmt --all -- --check`; `cargo check --workspace --locked`; `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`; `cargo test --workspace --locked`; product desktop FFmpeg/Xvfb smoke; repository governance. Run evidence, exact heads, PR review and post-merge main CI must be recorded after actual results.

**Manual**: MA-D-01 remains CURRENT/REQUIRED and MA-F-01…04 FUTURE/NOT READY. Do not mark PASS solely from CI.

## Evidence / checkpoint (2026-11-14)

- Implementation: [PR #34](https://github.com/wjhuang88/Laputa/pull/34) on `feature/laputa-001-f3a-direct-scenes`. New `DirectSceneEdit` actions use the canonical F1 validated edit path, reconstruct a fresh WorkspaceState, preserve valid Selection, and protect original source bytes. Actual GPUI controls bind these operations; preview epoch fences discard both stale render completions and obsolete cross-controller playback events. Render output paths are isolated per revision; initially launched product still supports the legacy `current.mp4` smoke fixture.
- First branch CI `#43464725773` failed **at `cargo fmt --all -- --check` after successful FFmpeg fixture**; 19 exact Rustfmt differences were applied in three files. This was a formatting-only failure, not a discovered runtime regression.
- An initial corrected F3a code head (up to `976ae10292a0863fdc225c89cced0b7b99659ed1`) passed [push CI #43465590098](https://github.com/wjhuang88/Laputa/actions/runs/43465590098) **success** and [PR CI #43465602800](https://github.com/wjhuang88/Laputa/actions/runs/43465602800) **success**.
- Subsequent changes align UI wording with the approved consumer-design vocabulary, correct stale README/MA-D-01 CLI examples against the actual binary argument parser, register the durable EVO-001 lesson, and add a separately documented F3a manual smoke. **Do not use the earlier code-head successes as exact-head merge evidence** after these commits.
- Headless tests cover move earlier/later, shortened duration, lock/unlock and deletion, reopened persisted film and Scene Strip/preview-plan projections, rejecting stale external film edits and invalid deletion, and comparing source bytes unchanged. Real product desktop user validation has **not** occurred.
- Manual checklist: `docs/acceptance/MANUAL-ACCEPTANCE.md` includes an **UNRUN F3a conditional preflight**. Story D MA-D-01 is CURRENT/REQUIRED; full Story F MA-F-01…04 is FUTURE/NOT READY. No checkboxes are completed and no credential-gated smoke has been performed.

## Disposition / next step

**Partial, not Story F completion.** F3a provides buttons, not pointer drag. F3b still must add true drag-and-drop reorder, finer trimming and a working input/submit path wired to F2 `RevisionRunner` with model-provider configuration. F4 generated “Try another” and MA-F/MVP real-desktop acceptance remain future work. The bounded F3a code has merged in PR #34 after exact-head push/PR CI success; post-merge main CI is independently tracked below. Issue #7 and Epic #1 remain Open. F3b/F4 and all real user checks remain pending.


## Final merge checkpoint (2026-11-14)

- PR #34 final head `6b0c2a0e014bf64298c3bcae9c318ad21a9bf187`: [push CI #43467717690](https://github.com/wjhuang88/Laputa/actions/runs/43467717690) **success**, [PR CI #43467727768](https://github.com/wjhuang88/Laputa/actions/runs/43467727768) **success**. No unresolved PR review threads; verified base/main expected SHA before squash merge.
- [PR #34](https://github.com/wjhuang88/Laputa/pull/34) **merged** as `0cf66df0c38b96e5b1e73384c0023039f32fa7c7`. Post-merge main CI [#43469951389](https://github.com/wjhuang88/Laputa/actions/runs/43469951389) is an independent gate; follow its actual GitHub result and update Issue #7, rather than inferring success from merge.
- A GitHub automation unexpectedly marked **Issue #7 Completed/Closed** on the partial-delivery PR merge; it was immediately **reopened** with all 4 original acceptance boxes unchecked. GitHub events confirm the close/reopen event sequence. See EVO-002; PR descriptions for partial slices must not include recognized issue-closing keywords even in negative sentences.
- F3a **manual real desktop preflight remains UNRUN**. The central acceptance ledger contains reproducible commands and checks for order, shorten, retain/guard and preview. The full MA-F-02 drag requirement is not satisfied by discrete move buttons. MA-D-01 remains CURRENT/REQUIRED.
