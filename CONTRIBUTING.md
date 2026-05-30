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
cargo deny check   # requires [cargo-deny](https://github.com/EmbarkStudios/cargo-deny)
```

5. Open a pull request against `master`. CI must pass (fmt, clippy, test, deny, release build).

## Pull request guidelines

- Keep changes scoped to one concern when possible.
- Update [README.md](README.md), [docs/](docs/), and help text if behavior or keybindings change.
- Add or extend tests for parsing, config, and non-UI logic.

## Releases

Maintainers tag releases as `v*` on `master`. Pushing a tag triggers the **Release** workflow (primary): binaries, checksums, and `tick.rb`. **CI** on push/PR runs fmt, clippy, tests, `cargo deny`, and a snapshot release build.

### Release checklist

1. Ensure `Cargo.toml` version matches the tag (e.g. `0.11.0` → `v0.11.0`).
2. Run `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, and `cargo test`.
3. Optionally `cargo deny check` (requires [cargo-deny](https://github.com/EmbarkStudios/cargo-deny)).
4. Merge to `master`; wait for CI green.
5. `git tag vX.Y.Z && git push origin vX.Y.Z`
6. Confirm the GitHub release has all platform binaries, `CHECKSUMS.txt`, and `tick.rb`.
7. Homebrew tap (choose one):
   - **Automated:** Add repo secret `HOMEBREW_TAP_TOKEN` (fine-grained PAT with `contents: write` on `homebrew-tick`). The release workflow pushes `tick.rb` to [homebrew-tick](https://github.com/aeswibon/homebrew-tick).
   - **Manual:** Copy `tick.rb` from the GitHub release into `Formula/tick.rb` and push the tap repo.
8. Smoke: `brew update && brew upgrade tick`, `tick --help`.

## Code of conduct

This project follows the [Contributor Covenant](CODE_OF_CONDUCT.md). Be respectful and constructive.
