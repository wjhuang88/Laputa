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

## Desktop application

`laputa-desktop` is the only application binary exposed by the `laputa-app` package. It is enabled through the `desktop-ui` feature.

Launch the desktop application with:

```sh
cargo run -p laputa-app --features desktop-ui --bin laputa-desktop
```

The desktop import path requires `ffprobe`, so install FFmpeg before exercising mixed-media import locally.

Never place provider credentials in tracked files. Use environment/OS-secret facilities selected by the implementation Story.
