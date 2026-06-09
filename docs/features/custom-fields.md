# Custom fields (detail)

Read custom fields in the table via `columns = ["customfield_10042", ...]`. Edit configured fields from the **Details** tab.

## Discover field ids (phase 2)

```bash
tick fields list --site my-team
tick fields list --site my-team --project HIN
```

JSON includes `id`, `name`, `suggested_type`, optional `options` (with `--project`), and a ready-made `config_snippet` for `config.toml`.

`tick --doctor` reminds you to run `fields list` per site.

## Config

```toml
[[detail.editable_fields]]
id = "customfield_10042"
label = "Story points"
type = "number"

[[detail.editable_fields]]
id = "customfield_10015"
label = "Start date"
type = "date"

[[detail.editable_fields]]
id = "customfield_10016"
label = "Review by"
type = "datetime"

[[detail.editable_fields]]
id = "customfield_10040"
label = "Approved"
type = "boolean"

[[detail.editable_fields]]
id = "customfield_10001"
label = "Environment"
type = "select"
options = ["Dev", "Staging", "Prod"]

[[detail.editable_fields]]
id = "customfield_10010"
label = "Tags"
type = "multiselect"
options = ["Frontend", "Backend", "Ops"]

[[detail.editable_fields]]
id = "customfield_10002"
label = "Reviewer"
type = "user"

# Resolve type + select options from Jira edit metadata per issue
[[detail.editable_fields]]
id = "customfield_10003"
label = "Team"
type = "auto"
```

| `type` | Edit UX |
|--------|---------|
| `text` | Footer prompt; empty clears the field |
| `number` | Footer prompt; validates as a number; empty clears |
| `date` | Footer prompt (`YYYY-MM-DD`); empty clears |
| `datetime` | Footer (`YYYY-MM-DD` or `YYYY-MM-DDTHH:MM`); empty clears |
| `boolean` | Yes/No picker |
| `select` | Picker from `options` (or from Jira when `options` omitted / `auto`) |
| `multiselect` | Checklist — **Space** toggles, **Enter** confirms; empty clears |
| `user` | User search picker (same as transition user fields) |
| `auto` | Fetches **editmeta** for the selected issue; supports all types above |

Field ids must be `customfield_<digits>`.

Editable fields are **fetched on refresh** even when not in `columns`.

## Keybinding

With the detail pane open: **`F`** — pick a field (or edit immediately when only one is configured).

See [CONFIGURATION.md](../CONFIGURATION.md#editable-custom-fields).
