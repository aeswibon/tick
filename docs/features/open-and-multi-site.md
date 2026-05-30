# Open issues and multi-site

## Open in browser

| Key | Context | Action |
|-----|---------|--------|
| `o` | Table | Open **selected** issue URL |
| `O` | Table | Open from **clipboard** or typed key/URL |

### `O` flow

1. Reads clipboard. If it contains `PROJ-123` or `https://….atlassian.net/browse/PROJ-123`, opens immediately.  
2. Otherwise footer input: paste key or URL, **`Enter`**.  
3. **Multi-site:** probes each `[[sites]]` in config order via `GET /rest/api/3/issue/{key}`; header shows `Checking site (2/3)…`; opens **first match**.  
4. Full browse URLs use the hostname to pick the site without probing.

### Example

```text
# clipboard: ENG-4042
O
# opens on the site where ENG-4042 exists

O
HIN-999
Enter
```

## Copy key (`y`)

Copies the selected issue key to the system clipboard (platform-dependent on Windows).

## Multi-site table

```toml
[[sites]]
name = "zeta"
base_url = "https://zeta-tm.atlassian.net"

[[sites]]
name = "corp"
base_url = "https://corp.atlassian.net"
boards = { CORP = 3 }
```

- **site** column shows which instance each row came from.  
- All actions use that row’s `base_url`.  
- Per-site ticket counts appear in the header.

## Site errors (`!`)

When some sites fail on fetch:

- Footer: `N site error(s) — press ! for details`  
- **`!`** — scrollable overlay with per-site messages  
- **`j`/`k`** scroll; **`!`** or **`Esc`** close  

Successful sites still contribute tickets; failed sites do not clear the whole table.

## Related

- [auth-cli-cache.md](auth-cli-cache.md) — partial failure behavior
- [views-and-tabs.md](views-and-tabs.md)
