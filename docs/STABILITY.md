# Stability policy (1.0+)

**tick 1.0.0** is the first release we treat as **stable** for everyday use and scripting.

## Versioning

We follow [Semantic Versioning](https://semver.org/):

| Bump | When |
|------|------|
| **1.0.x** | Bug fixes only; no intentional breaking changes |
| **1.x.0** | New features, backward compatible |
| **2.0.0** | Breaking changes (with migration notes in CHANGELOG) |

Pre-1.0 releases (`0.12`–`0.27`) were rapid iteration; **1.x** commits to slower, documented change.

## Stable surfaces

These are covered by the 1.x stability promise:

| Surface | Notes |
|---------|--------|
| **`config.toml`** | Existing keys and `[[sites]]` / `[[hooks.*]]` / `[[detail.editable_fields]]` shapes |
| **Headless CLI** | `tick issue`, `tick search`, `tick bulk` — JSON field names on success objects |
| **Hook env vars** | `TICK_VIEW`, `TICK_JSON_PATH`, `TICK_BULK_*`, `TICK_CONFIG_PATH`, `TICK_CHECK_*`, `TICK_KEY`, `TICK_SITE`, etc. |
| **Plugin manifest** | `api = "1"` with capabilities `filter_tickets`, `on_key`, `run_transition` |
| **Lua `tick` table** | `version`, `view`, `tickets`, `selected`, transition helpers when enabled |

## Not guaranteed stable in 1.x

| Area | Expectation |
|------|-------------|
| **New CLI subcommands** | Additive minors are OK |
| **New plugin capabilities** | Opt-in manifest flags only |
| **TUI keybindings** | Documented in [KEYBINDINGS.md](KEYBINDINGS.md); new keys may be added |
| **Undocumented internals** | Rust crate layout, private modules |

## Deprecations

Breaking changes to stable surfaces require:

1. Deprecation notice in CHANGELOG and docs for at least one **minor** release
2. Runtime warning where practical (`tick --doctor` or footer)
3. Migration guide in CHANGELOG for **2.0** (or the breaking minor if unavoidable)

## Reporting issues

- Bugs: [GitHub Issues](https://github.com/aeswibon/tick/issues)
- Security: [SECURITY.md](../SECURITY.md) (no public issues for vulnerabilities)
