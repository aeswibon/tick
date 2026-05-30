# Status and workflow transitions

Jira Cloud does **not** allow setting `status` directly. tick loads **workflow transitions** for the selected issue and applies one via the REST API.

## Keys

| Key | Context | Action |
|-----|---------|--------|
| `t` or `T` | Table or detail | Open transition picker |
| `j` / `k` | Picker | Move selection |
| `Enter` | Picker | Apply transition |
| `1`–`9` | Picker | Pick transition by number |
| `Esc` | Picker / field modal | Cancel |

**Note:** In the **create wizard**, `t` reopens the issue-type picker instead of status — only when `create_session` is active.

## What you see

Each option is shown as **action → target status**, for example:

```text
Start Progress → In Progress
Done → Done
```

That matches Jira’s workflow, not a flat status list.

## Required fields

Some transitions require **Resolution**, **assignee**, or custom fields. tick reads `transitions[].fields` from Jira and prompts before POST.

| Field type | UI |
|------------|-----|
| Resolution, priority, option lists | Picker (`j`/`k`, `Enter`, `1`–`9`) |
| User (assignee, etc.) | Footer filter + cached user list |
| Boolean | Yes / No picker |
| Date | Footer: `YYYY-MM-DD` |
| Date-time | Footer: `YYYY-MM-DD` or `YYYY-MM-DDTHH:MM` |
| Text / number | Footer input |

### User field: filter vs load more

| Input | Effect |
|-------|--------|
| Type in footer | Filter cached assignable users locally |
| `j` / `k` | Move in picker list (does **not** append to search text) |
| **⌘R** (macOS) or **Ctrl+R** | Fetch more users from Jira and **merge** into cache (up to 500) |
| Plain `r` / `R` | Types the letter into the filter (e.g. names starting with R) |

### Example: close with resolution

```text
t                    # open transitions
j j                  # select "Done → Done"
Enter                # if resolution required, picker opens
j                    # select "Fixed"
Enter                # POST transition; table refreshes
```

If Jira rejects the transition, tick parses `errors` / `errorMessages` and re-prompts for missing fields.

## Errors you might see

| Message | Cause |
|---------|--------|
| Select a ticket to change status | No row selected |
| No workflow transitions | Closed issue, permissions, or no transitions from current status |
| resolution required | Pick resolution in the follow-up dialog |

## Related

- [KEYBINDINGS.md](../KEYBINDINGS.md#overlays--transition-required-fields)
- [CONFIGURATION.md](../CONFIGURATION.md)
