# Detail pane

Press **`Enter`** on a row to split the screen: **~60% table**, **~40% detail**. Press **`Enter`** or **`Esc`** again to close.

## Tabs inside detail

| Key | Action |
|-----|--------|
| `h` | Previous tab |
| `l` | Next tab |

**Exception:** on the **Closed** tab with detail **closed**, `h` toggles assignee history scope — not detail tabs.

| Tab | Content |
|-----|---------|
| **Details** | Key, link, type, status, priority, dates, people, labels, sprint, parent |
| **Description** | Jira ADF rendered in the terminal (lazy-loaded) |
| **Comments** | Thread with author, date, ADF body (lazy-loaded) |
| **Links** | Issue links and subtasks — see [issue-links-subtasks.md](issue-links-subtasks.md) |

## Lazy load (description and comments)

View refresh fetches **table fields only** for every row. Description and comments load **on demand** when:

- You open the detail pane (`Enter`)
- You move to another issue with detail open (`j` / `k`, except on the Links tab)
- You switch to the **Description** or **Comments** tab (`h` / `l`)

While loading, those tabs show `Loading description…` or `Loading comments…`.

If the fetch fails (permissions, rate limit, network), the tab shows the error and **`Press r to refresh`** instead of spinning forever. The footer also shows the error in red. A full view refresh (`r`) clears cached detail state so tick can retry.

Headless **`tick issue show`** still returns full body in JSON without opening the TUI.

See also [platform.md](platform.md#lazy-detail-load).

## Footer notices

Short success messages (e.g. “Watching PROJ-123”, bulk batch results) clear automatically after **`notice_secs`** in config (default **5**). Set `notice_secs = 0` to keep notices until the next action. Errors are not auto-cleared.

## Reading rich content (ADF)

tick converts Jira **Atlassian Document Format** to terminal-friendly text:

- Headings, bullets, numbered lists, task lists  
- **Bold**, code, links, emoji  
- `@mentions` as display names  
- Tables, media, expand blocks (labeled when unsupported)

### Exotic blocks when editing

If you press **`D`** to edit description, rare block types may appear as fenced **` ```adf-json`** sections. Do not delete those fences unless you intend to replace the block — they round-trip verbatim on save.

## Config shortcut

| Key | Action |
|-----|--------|
| `e` | Open `~/.config/tick/config.toml` in the system editor |

Works with detail open or closed.

## Writes from detail

All edits refresh views after success. Errors appear in the footer (red).

| Key | Guide |
|-----|--------|
| `t` / `T` | [status-transitions.md](status-transitions.md) |
| `c` | [comments-and-worklogs.md](comments-and-worklogs.md) |
| `w` | [comments-and-worklogs.md](comments-and-worklogs.md) |
| `a` / `u` | Assign to me / unassign |
| `S` | [editing-fields.md](editing-fields.md) — summary |
| `P` | [editing-fields.md](editing-fields.md) — priority |
| `L` | [editing-fields.md](editing-fields.md) — labels |
| `M` | [editing-fields.md](editing-fields.md) — sprint |
| `D` | [editing-fields.md](editing-fields.md) — description |
| `F` | [custom-fields.md](custom-fields.md) — configured custom fields |

## Related

- [KEYBINDINGS.md](../KEYBINDINGS.md#detail-pane-open)
