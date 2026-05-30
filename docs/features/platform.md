# Platform: config reload and rate limits

v0.11.0 quality-of-life for long-running tick sessions.

## Config reload (`R`)

Edit `~/.config/tick/config.toml`, save, then press **`R`** in the table (detail closed).

Reload applies:

- `[[sites]]`, `[views]`, `[[views.custom]]`
- `[[create.templates]]` and `create.templates_file`
- `columns`, `theme`, `max_results`, `page_size`
- API token / OAuth client (re-authenticates)

**Not reset:** current ticket list, selection, or open detail pane. Press **`r`** after reload to fetch fresh data.

### Workflow

```text
e                # open config in $EDITOR
# add [[views.custom]] or change JQL
# save and quit editor
R                # reload in tick
r                # refresh active view
```

## Jira rate limits (HTTP 429)

tick already retries 429/5xx with exponential backoff. When Jira rate-limits you, the **footer** shows:

```text
Jira rate limit — wait ~Ns, then r to retry
```

Wait for the countdown, then press **`r`**. Avoid hammering **`r`** while the message is visible.

## JQL pagination (`max_results`)

`max_results` in `config.toml` is the **total cap per site** for each view fetch (not “one page of 100”).

When a view matches more issues than one JQL page allows, tick:

1. Pages `POST /rest/api/3/search/jql` with `maxResults = min(remaining, 100)` until the cap or Jira reports `isLast`
2. Chunks `bulkfetch` in batches of 100 issue ids
3. Updates the **footer** during multi-page search (`JQL search page 2 (100/500)…`) so the TUI stays responsive

Raise `max_results` (default 500, max 5000) if you need larger views; Jira rate limits still apply.

## Lazy detail load

View refresh (`r`, tab switch, background refresh) fetches **table fields only** — not description or comments for every row.

When you **open the detail pane** (`Enter`) or move to another issue with detail open, tick loads description and comments for that issue only. Description and Comments tabs show “Loading…” until the fetch completes.

Headless **`tick issue show`** still includes full issue body in JSON.

## Related

- [CONFIGURATION.md](../CONFIGURATION.md)
- [KEYBINDINGS.md](../KEYBINDINGS.md)
- [auth-cli-cache.md](auth-cli-cache.md)
