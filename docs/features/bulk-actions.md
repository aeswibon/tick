# Bulk table actions

Mark multiple issues on the main table and apply one action without opening the detail pane.

## Selection

| Key | Action |
|-----|--------|
| `Space` | Toggle mark on the current row |
| `Shift+Space` | Mark all **filtered** visible rows (up to 50) |
| `Esc` | Clear all marks (table focused, no detail open) |

Marked rows show `✓` in the **Key** column. The footer shows how many are selected.

**Limit:** at most **50** issues per bulk operation.

## Same-site rule

Bulk **transition**, **assign**, and **watch** require every marked issue to be on the **same** `[[sites]]` entry. Mixed sites show: `Bulk actions require a single site`.

## Bulk transition (`t`)

1. Mark one or more rows.
2. Press `t` (same as single-issue status change).
3. Pick a transition from the modal (loaded from the **first** marked issue).
4. tick applies that transition **by name** to each marked issue.

**Results:** footer notice like `Bulk transition: 4 ok, 1 failed (PROJ-9: transition requires fields…)`.

**Limitations:**

- Transitions that need extra fields (resolution, assignee, etc.) **fail** for that issue in bulk mode. Use single-issue `t` on the detail/table selection for those.
- Matching is by transition **name** (workflow action), not target status id.

## Bulk assign (`a`)

With marks active on the table (detail closed), `a` assigns the **current Jira user** to every marked issue on that site.

## Bulk labels (`L`)

With marks on the table (detail closed), `L` opens a footer prompt for **comma-separated** labels. The value **replaces** labels on every marked issue (same as single-issue `L` in the detail pane). Empty input clears labels.

**Results:** footer notice like `Bulk labels: 4 ok, 1 failed (PROJ-9: …)`.

Same-site rule and 50-issue cap apply.

## Bulk watch (`W` / `Shift+W`)

With marks on the table (detail closed):

| Key | Action |
|-----|------|
| `W` | Watch all marked issues |
| `Shift+W` | Unwatch all marked issues |

Only the **active tab** is refreshed afterward (not every background view).

## Related

- [table-and-navigation.md](table-and-navigation.md) — row selection and filtering
- [status-transitions.md](status-transitions.md) — single-issue transitions
- [KEYBINDINGS.md](../KEYBINDINGS.md)
