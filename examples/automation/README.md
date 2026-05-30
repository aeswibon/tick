# tick automation examples

Sample shell hooks for `config.toml`. Copy scripts to `~/.local/bin/` (or another path on your `PATH`) and point `command` at them.

## Bulk complete

`on-bulk-complete.sh` logs bulk results and exits non-zero when any issue failed (useful for chaining in CI).

```toml
[[hooks.on_bulk_complete]]
command = "~/.local/bin/on-bulk-complete.sh"
```

Requires `jq` for the sample script. Env vars: `TICK_BULK_LABEL`, `TICK_JSON_PATH`, `TICK_OK_COUNT`, `TICK_FAIL_COUNT`.

## Refresh

For `[[hooks.on_refresh]]`, use env `TICK_VIEW`, `TICK_JSON_PATH`, `TICK_ISSUE_COUNT` — see [docs/features/automation.md](../../docs/features/automation.md).
