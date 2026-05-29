# Keybindings reference

## Global (table focused)

| Key | Action |
|-----|--------|
| `j` / `k` or `↑` / `↓` | Move selection up / down |
| `g` / `G` | First / last row in filtered list |
| `[` / `]` | Scroll viewport by `page_size` rows |
| `Enter` | Toggle detail pane |
| `Esc` | Close pane, overlay, or cancel input |
| `?` | Help overlay |
| `/` | Filter tickets (matches key, summary, status, assignee, labels, sprint, parent) |
| `s` | Cycle sort field: default → age → priority → status → key |
| `S` | Toggle sort **asc** ↑ / **desc** ↓ (table only; not in detail pane) |
| `r` | Refresh current view from Jira |
| `y` | Copy issue key to clipboard |
| `o` | Open issue in browser |
| `e` | Open config file in editor |
| `t` | Status transition picker |
| `!` | Site errors overlay (when warnings exist) |
| `←` / `→` or `Tab` / `Shift+Tab` | Cycle view tab |
| `1`–`5` | Jump to Assigned / Updated / Mentions / Watched / Sprint |
| `q` | Quit |

## Detail pane open

| Key | Action |
|-----|--------|
| `h` / `l` | Previous / next tab (Details · Description · Comments) |
| `c` | Add comment (`@` opens user tag picker) |
| `w` | Log work (e.g. `30m`, `1h`) |
| `a` / `u` | Assign to me / unassign |
| `S` | Edit **summary** |
| `P` | Edit **priority** (picker) |
| `L` | Edit **labels** (comma-separated, replaces all) |
| `M` | **Move sprint** / backlog (Agile board) |
| `D` | Edit **description** (markdown + `@` mentions) |

## Overlays

| Context | Keys |
|---------|------|
| Transition picker | `j`/`k`, `Enter`, `1`–`9`, `Esc` |
| Priority picker | `j`/`k`, `Enter`, `1`–`9`, `Esc` |
| Sprint picker | `j`/`k`, `Enter`, `1`–`9`, `Esc` |
| Site errors | `j`/`k`, `!` or `Esc` close |
| Filter mode | Type filter, `Enter`/`Esc` apply |
| Comment `@` picker | `j`/`k`, `Enter` pick user, `Esc` close picker |
| Input modes (`c`, `w`, `S`, `L`, `D`) | Type text, `Enter` submit, `Esc` cancel |
