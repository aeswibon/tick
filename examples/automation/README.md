# tick automation examples

Shell and CI recipes for headless `tick` and `config.toml` hooks. Copy scripts to `~/.local/bin/` (or another path on your `PATH`) and adjust `--site` / JQL for your workspace.

| File | Purpose |
|------|---------|
| [on-bulk-complete.sh](on-bulk-complete.sh) | `[[hooks.on_bulk_complete]]` — log failures; exit 1 if any issue failed |
| [on-refresh-slack.sh](on-refresh-slack.sh) | `[[hooks.on_refresh]]` — example: post issue count (replace with your notifier) |
| [on-config-reload.sh](on-config-reload.sh) | `[[hooks.on_config_reload]]` — log `tick --check` findings after **R** |
| [on-mark.sh](on-mark.sh) | `[[hooks.on_mark]]` — log when **Space** marks a row |
| [cron-assignee-report.sh](cron-assignee-report.sh) | Cron-friendly: list your open issues (requires `jq`) |
| [ci-transition.sh](ci-transition.sh) | Transition one issue by key + transition name (CI entrypoint) |
| [github-actions-transition.yml](github-actions-transition.yml) | Sample GitHub Actions job |

Docs: [docs/features/automation.md](../../docs/features/automation.md).

## Bulk complete

```toml
[[hooks.on_bulk_complete]]
command = "~/.local/bin/on-bulk-complete.sh"
```

Requires `jq` for the sample script. Env: `TICK_BULK_LABEL`, `TICK_JSON_PATH`, `TICK_OK_COUNT`, `TICK_FAIL_COUNT`.

## Refresh

```toml
[[hooks.on_refresh]]
command = "~/.local/bin/on-refresh-slack.sh"
views = ["assigned"]
```

Env: `TICK_VIEW`, `TICK_JSON_PATH`, `TICK_ISSUE_COUNT`.

## Config reload

```toml
[[hooks.on_config_reload]]
command = "~/.local/bin/on-config-reload.sh"
```

Env: `TICK_CONFIG_PATH`, `TICK_JSON_PATH`, `TICK_CHECK_ERRORS`, `TICK_CHECK_WARNS`.

## Mark

```toml
[[hooks.on_mark]]
command = "~/.local/bin/on-mark.sh"
```

Env: `TICK_KEY`, `TICK_SITE`, `TICK_JSON_PATH` (single issue). Not fired for unmark or Shift+Space mark-all.

## Cron

```cron
0 8 * * 1-5  /home/you/.local/bin/cron-assignee-report.sh --site zeta >> ~/tick-standup.txt 2>&1
```

## CI

```bash
export TICK_CONFIG=/path/to/config.toml
./examples/automation/ci-transition.sh HIN-123 "Done" zeta
```

Or use the GitHub Actions workflow snippet as a starting point.
