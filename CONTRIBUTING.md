# Contributing to tick

Thanks for your interest in improving tick.

## Development setup

1. Install [Rust](https://rustup.rs/) (2021 edition).
2. Clone the repository and run `cargo build`.
3. Create a local config: `cargo run -- --init`, then edit `~/.config/tick/config.toml`.

## Workflow

1. Open an issue or comment on an existing one before large changes.
2. Create a branch from `master`.
3. Make focused commits with clear messages.
4. Run checks locally before opening a PR:

```bash
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```

5. Open a pull request against `master`. CI must pass.

## Pull request guidelines

- Keep changes scoped to one concern when possible.
- Update README and help text if behavior or keybindings change.
- Add or extend tests for parsing, config, and non-UI logic.

## Releases

Maintainers tag releases as `v*` on `master`. Pushing a tag triggers the **Release** workflow (primary): binaries, checksums, and `tick.rb`. **CI** on push/PR runs a snapshot `cargo test` + `cargo build --release` only.

## Code of conduct

This project follows the [Contributor Covenant](CODE_OF_CONDUCT.md). Be respectful and constructive.
