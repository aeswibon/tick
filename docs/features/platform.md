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

## Related

- [CONFIGURATION.md](../CONFIGURATION.md)
- [KEYBINDINGS.md](../KEYBINDINGS.md)
- [auth-cli-cache.md](auth-cli-cache.md)
