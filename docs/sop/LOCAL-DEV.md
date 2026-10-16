# Local Development

## Toolchain

Laputa targets Rust 1.95 (see `rust-toolchain.toml`). Talos is pinned in the workspace manifest.

## Baseline commands

```sh
cargo fmt --all -- --check
cargo check --workspace --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked
```

The repository currently contains initialization scaffolding only. As runtime/GUI dependencies are wired, update this document with verified launch commands rather than inventing them in advance.

Never place provider credentials in tracked files. Use environment/OS-secret facilities selected by the implementation Story.
