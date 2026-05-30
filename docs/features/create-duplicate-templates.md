# Create, duplicate, and templates

Create and duplicate work from the **table** (detail pane can be open or closed). Press **`Esc`** anytime to cancel the wizard.

## Quick reference

| Key | Action |
|-----|--------|
| `n` | New issue — blank wizard |
| `N` | New issue from **config template** |
| `C` | **Duplicate** selected issue |
| `X` | **Export** selected issue as a new template |
| `Shift+E` | **Manage** templates (edit fields, delete) |
| `p` | During wizard: re-pick project |
| `t` | During wizard: re-pick issue type (not status) |
| `Ctrl+P` | While editing create description: toggle markdown preview |

## New issue (`n`)

### Flow

1. Site (if multiple `[[sites]]`)  
2. Project picker (skipped if `create_project` set)  
3. Issue type picker (skipped if `create_issue_type` set)  
4. Summary (required)  
5. Description (optional markdown) — **`Ctrl+P`** toggles live preview (same rendering as the detail Description tab)  
6. Required custom fields from create metadata (same UI as transition fields)

### Config shortcuts

```toml
[[sites]]
name = "zeta"
base_url = "https://zeta-tm.atlassian.net"
create_project = "ENG"
create_issue_type = "Task"
```

## Templates (`N`)

Pre-defined issues in config. Minimal typing at create time.

```toml
[create]
clone_summary_prefix = "Copy of: "
templates_file = "templates/local.toml"   # optional; merged at load

[[create.templates]]
name = "hin-bug"
site = "zeta"
project = "HIN"
issue_type = "Bug"
summary = "Bug: "
description = '''## Steps

1.

## Expected

'''
labels = ["support"]
priority = "Medium"
# assignee_account_id = "712020:..."
# parent_key = "HIN-100"
# extra_fields = { customfield_10020 = 123 }
```

### Example

```text
N                    # template picker
Enter                # pick "hin-bug"
                     # edit summary in footer
Fix login on SSO
Enter                # optional description, then required fields
```

## Duplicate (`C`)

Copies from the **selected** row (re-fetches full issue for accurate IDs):

- Project, type, labels, priority, assignee, due date, parent, description, sprint (if configured)  
- Summary: `[create].clone_summary_prefix` + original summary (default `Copy of: `)

```toml
[[sites]]
create_clone_link = true
clone_link_type = "Cloners"
```

After create, optionally links new issue → source as **Cloners**.

## Export template (`X`)

Save the selected issue into config for reuse with **`N`**.

### Steps

1. **`X`** on a selected ticket — loads full issue from Jira.  
2. **Fields to save** — `Space` toggles include (summary, description, labels, priority, assignee, parent, sprint, due date). `Enter` continues.  
3. **Clear values** — for included fields, `Space` marks “empty when creating from template”. `Enter` continues.  
4. **Name** — footer template id (e.g. `hin-471632-payment-fix`). `Enter` saves.

Saved to:

- `[[create.templates]]` in `config.toml`, or  
- `create.templates_file` if set (e.g. `templates/zeta.toml`)

Footer confirms path; template is available immediately via **`N`**.

### Example export workflow

```text
1                    # Assigned tab
j j                  # select HIN-471632
X
Space Space          # toggle off assignee and parent
Enter
Enter                # keep values where needed
hin-payment-template
Enter
```

## CLI template export

Bootstrap templates without the TUI:

```bash
tick template export zeta HIN-471632 CSJR-25019 HUAT-18652 \
  -o ~/.config/tick/templates/zeta.toml

# Append to an existing file
tick template export zeta HIN-100 --output templates/more.toml --append
```

Then reference the file in config:

```toml
[create]
templates_file = "templates/zeta.toml"
```

Or paste blocks into `[[create.templates]]` in `config.toml`.

## Manage templates (`Shift+E`)

Edit or remove templates without leaving the TUI. See [saved-views-templates-columns.md](saved-views-templates-columns.md#template-manager-shifte).

```text
Shift+E → pick template → e/p/i/b/l to edit → d to delete
```

Writes to `create.templates_file` when set, otherwise updates `[[create.templates]]` in `config.toml`.

## Related

- [CONFIGURATION.md](../CONFIGURATION.md)
- [KEYBINDINGS.md](../KEYBINDINGS.md#create--duplicate--templates)
