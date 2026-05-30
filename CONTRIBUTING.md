# Contributing to tick

Thanks for your interest in improving tick.

## Development setup

1. Install [Rust](https://rustup.rs/) (2021 edition).
2. Clone the repository and run `cargo build`.
3. Create a local config: `cargo run -- --init`, then edit `~/.config/tick/config.toml`.

Read [docs/architecture/README.md](docs/architecture/README.md) for module layout and the event loop.

## Your first PR

### 1. Fork and branch

```bash
git checkout -b fix/my-change
```

For large features, open an issue first so we can align on scope.

### 2. Run checks locally

Same as CI:

```bash
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test
cargo deny check   # cargo install cargo-deny
```

Optional: `cargo bench` for performance baselines.

### 3. Test without a live Jira site

You do **not** need Jira Cloud for most changes:

| Change type | How to test |
|-------------|-------------|
| Parsing, config, JQL, keys | `cargo test` (unit + [proptest](docs/architecture/testing.md)) |
| HTTP client / API shapes | [wiremock](docs/architecture/testing.md) tests in `src/api/` |
| Help text / static UI copy | `INSTA_UPDATE=1 cargo test ui::snapshots` after intentional edits |

Use a **developer Cloud site** only for manual TUI QA (create `tick --init`, API token in `~/.config/tick/token`).

### 4. Common gotchas

| Topic | Note |
|-------|------|
| **Auth** | API token (`email` + `token` in config or `TICK_TOKEN`) vs OAuth (`tick auth login` → `oauth.json`). `--doctor` checks connectivity. |
| **Config path** | `~/.config/tick/config.toml` (or platform equivalent). `tick --init` creates a starter file. |
| **Rate limits** | Jira **429** triggers backoff; footer shows wait time. Tests use wiremock, not live limits. |
| **Closed tab** | Tab `6` runs server-side JQL search; other tabs use local `/` filter. |
| **UI keys** | Case-sensitive (`S` ≠ `s`). Update [docs/KEYBINDINGS.md](docs/KEYBINDINGS.md) and `src/ui/help.rs` when adding bindings. |

### 5. Open the PR

- Target branch: **`master`**
- Describe what changed and how you tested it
- Keep diffs focused; update user docs when behavior changes

## Workflow

1. Open an issue or comment on an existing one before large changes.
2. Create a branch from `master`.
3. Make focused commits with clear messages.
4. Run checks locally before opening a PR (see above).
5. Open a pull request against `master`. CI must pass (fmt, clippy, test, deny, release build).

## Pull request guidelines

- Keep changes scoped to one concern when possible.
- Update [README.md](README.md), [docs/](docs/), and help text if behavior or keybindings change.
- Add or extend tests for parsing, config, and non-UI logic (see [testing guide](docs/architecture/testing.md)).

## Community

- **Bug reports / features:** [GitHub Issues](https://github.com/aeswibon/tick/issues)
- **Discussions:** enable in repo settings for Q&A and ideas (maintainers)

## Releases

Maintainers tag releases as `v*` on `master`. Pushing a tag triggers the **Release** workflow (primary): binaries, checksums, and `tick.rb`. **CI** on push/PR runs fmt, clippy, tests, `cargo deny`, and a snapshot release build.

Use the [**Release checklist** issue template](.github/ISSUE_TEMPLATE/release_checklist.yml) when cutting a version.

### Release checklist

1. Ensure `Cargo.toml` version matches the tag (e.g. `0.12.0` → `v0.12.0`).
2. Update [CHANGELOG.md](CHANGELOG.md).
3. Run `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, and `cargo test`.
4. Optionally `cargo deny check` (required in CI).
5. Merge to `master`; wait for CI green.
6. `git tag vX.Y.Z && git push origin vX.Y.Z`
7. Confirm the GitHub release has all platform binaries, `CHECKSUMS.txt`, and `tick.rb`.
8. Homebrew tap (choose one):
   - **Automated:** Add repo secret `HOMEBREW_TAP_TOKEN` (fine-grained PAT with `contents: write` on [homebrew-tick](https://github.com/aeswibon/homebrew-tick)).
   - **Manual:** Copy `tick.rb` from the GitHub release into `Formula/tick.rb` and push the tap repo.
9. Smoke: `brew update && brew upgrade tick`, `tick --help`.

## Code of conduct

This project follows the [Contributor Covenant](CODE_OF_CONDUCT.md). Be respectful and constructive.
