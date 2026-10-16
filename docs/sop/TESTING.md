# Testing Policy

Testing follows risk and observable acceptance, not file count.

## Required layers

- Domain/provider contracts: deterministic unit tests for data validation, routing boundaries and failure semantics.
- Tool/runtime integration: fake providers and deterministic fixtures before live paid-provider tests.
- Media pipeline: probe/render/hash fixtures proving originals remain unchanged and outputs are valid/playable.
- GUI behavior: focused interaction tests where possible plus manual/visual smoke evidence for playback, selection, drag/trim/lock flows.
- External APIs: opt-in live smoke tests only when credentials exist; default CI must not depend on paid/network providers.

## Hard gates

- Any operation touching imported media must include evidence that supported paths do not overwrite originals.
- Any asynchronous generation path must test submitted/running/failed/cancelled/completed semantics; timeline insertion requires verified completion.
- Structured external AI output must be schema-validated and malformed output tested.
- Autonomous repair/generation loops require explicit retry/cost/time bounds and termination tests.

## Baseline commands

```sh
cargo fmt --all -- --check
cargo check --workspace --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked
scripts/validate_project_governance.sh .
```

Record actual results in the active iteration. If an environment lacks Rust/tooling or credentials, mark that check unrun and keep closure partial when the check is required by the Story.
