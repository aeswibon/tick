# Custom fields (detail)

Read custom fields in the table via `columns = ["customfield_10042", ...]`. Edit configured fields from the **Details** tab.

## Config

```toml
[[detail.editable_fields]]
id = "customfield_10042"
label = "Story points"
type = "text"

[[detail.editable_fields]]
id = "customfield_10001"
label = "Environment"
type = "select"
options = ["Dev", "Staging", "Prod"]

[[detail.editable_fields]]
id = "customfield_10002"
label = "Reviewer"
type = "user"
```

| `type` | Edit UX |
|--------|---------|
| `text` | Footer prompt; empty clears the field |
| `select` | Picker from `options` |
| `user` | User search picker (same as transition user fields) |

Field ids must be `customfield_<digits>` (discover with `tick --doctor`).

Editable fields are **fetched on refresh** even when not in `columns`.

## Keybinding

With the detail pane open: **`F`** — pick a field (or edit immediately when only one is configured).

See [CONFIGURATION.md](../CONFIGURATION.md#editable-custom-fields).
