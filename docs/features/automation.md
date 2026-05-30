# Automation

Headless commands, config hooks, and Lua plugins share the same `config.toml`, auth, and Jira client as the TUI. Pick the layer that matches where your logic runs.

## Choose a layer

| Need | Use | Why |
|------|-----|-----|
| Cron, CI, Slack bot, one-off shell | **`tick` CLI** | Separate process; easy to test; no TUI required |
| React when a view refreshes or bulk finishes in the TUI | **Hooks** (`[[hooks.on_refresh]]`, `[[hooks.on_bulk_complete]]`) | tick passes JSON on disk + env vars; no plugin runtime |
| Filter table rows or bind keys inside tick | **Lua plugins** | In-process; read `tick.tickets` / `tick.selected`; optional `run_transition` |
| Custom fields, complex transitions, admin | **Jira REST** (or Automation app) | tick does not expose every Jira API |

**Parity:** Bulk transition/assign/labels in the TUI follow the same rules as `tick bulk` and `tick issue transition` (simple transitions only; required-field modals stay in the TUI).

## `tick issue show`

```bash
tick issue show HIN-123 --site zeta
```

Prints JSON: key, summary, status, assignee, labels, url.

## `tick issue transition`

```bash
tick issue transition HIN-123 --to "In Progress" --site zeta
```

Applies a workflow transition **by name**. Fails if the transition requires extra fields (use the TUI for those).

## `tick search`

```bash
tick search --jql 'project = HIN AND assignee = currentUser() ORDER BY updated DESC' --site zeta
```

Prints JSON: `{ "issues": [...], "warnings": [...] }`. Respects `max_results` in config.

### jq recipes

Issue list (keys only):

```bash
tick search --jql 'assignee = currentUser() AND resolution = Unresolved' --site zeta \
  | jq -r '.issues[].key'
```

Summaries for a standup paste:

```bash
tick search --jql 'project = HIN AND status = Blocked' --site zeta \
  | jq -r '.issues[] | "\(.key): \(.summary) [\(.status)]"'
```

Count + warnings:

```bash
tick search --jql 'updated >= -1d' --site zeta \
  | jq '{ count: (.issues | length), warnings: .warnings }'
```

Single issue from `issue show`:

```bash
tick issue show HIN-42 --site zeta | jq '{ key, status, assignee, labels }'
```

Bulk result (after `tick bulk`):

```bash
tick bulk transition --site zeta --keys HIN-1,HIN-2 --to Done \
  | jq '{ ok: [.ok[].key], failed: .failed }'
```

## `tick bulk`

All bulk commands require `--site` and `--keys` (comma-separated or repeated).

```bash
tick bulk transition --site zeta --keys HIN-1,HIN-2 --to Done
tick bulk assign --site zeta --keys HIN-1,HIN-2 --me
tick bulk labels --site zeta --keys HIN-1 --set "bug,triage"
```

Output is JSON with `ok` and `failed` arrays. Exit code `1` if any issue fails. Use `--quiet` for compact JSON on stdout only.

## Cron

Run from the same user account that owns `~/.config/tick/config.toml` (or set `TICK_CONFIG`).

Daily assignee report (keys + summary to a file):

```bash
0 8 * * 1-5  /path/to/examples/automation/cron-assignee-report.sh >> ~/tick-standup.txt 2>&1
```

Transition stale “In Review” tickets (dry-run first by echoing the command):

```bash
0 18 * * 5   tick bulk transition --site zeta --keys "$(tick search --jql '...' --site zeta | jq -r '.issues[].key' | paste -sd,)" --to Done
```

Prefer a small wrapper script (see `examples/automation/`) so cron logs failures and you can add `--quiet` + alerting.

## CI (GitHub Actions)

Transition when a PR merges (issue key in branch name or env):

```yaml
# See examples/automation/github-actions-transition.yml
- run: tick issue transition "$JIRA_KEY" --to "Done" --site zeta
  env:
    TICK_CONFIG: ${{ github.workspace }}/.tick/config.toml
```

Store API token or OAuth refresh material in GitHub **secrets**; never commit credentials. Use `tick --doctor` in a workflow job to verify auth before writes.

## Config validation

```bash
tick --check    # offline structural validation
tick --doctor   # live Jira API probes
```

## Refresh hooks

After a **successful** refresh of the active view (`r` or background update), tick can run shell commands:

```toml
[[hooks.on_refresh]]
command = "~/.local/bin/on-tick-refresh.sh"
views = ["assigned", "mentions"]   # optional; default = all views
timeout_secs = 30                  # optional; default 30
```

| Variable | Content |
|----------|---------|
| `TICK_VIEW` | View id: `assigned`, `mentions`, `watched`, `updated`, `sprint`, `closed`, or custom view **name** |
| `TICK_JSON_PATH` | Path to a temp JSON file (array of issues) |
| `TICK_ISSUE_COUNT` | Number of issues |

Hooks run in the background. Failures print to stderr as `[tick hook] …`. Not run when the fetch returns site errors.

Details: [CONFIGURATION.md](../CONFIGURATION.md#refresh-hooks).

## Bulk-complete hooks

After **TUI** bulk actions (assign, labels, transition, watch/unwatch) or **`tick bulk`** finishes, tick can run shell commands:

```toml
[[hooks.on_bulk_complete]]
command = "~/.local/bin/on-tick-bulk.sh"
timeout_secs = 30   # optional; default 30
```

| Variable | Content |
|----------|---------|
| `TICK_BULK_LABEL` | Action label, e.g. `Bulk assign`, `Bulk transition` |
| `TICK_JSON_PATH` | Temp JSON: `{ "label", "ok", "failed": [{ "key", "error" }] }` |
| `TICK_OK_COUNT` | Successful issue count |
| `TICK_FAIL_COUNT` | Failed issue count |

Hooks run even when some issues fail (so you can alert or log partial failures). Same background + stderr `[tick hook] …` behavior as refresh hooks.

Example scripts: [examples/automation/](../../examples/automation/).

## Plugins (Lua)

For in-process table filters, key chords, and simple transitions from Lua, see [plugins.md](plugins.md).
