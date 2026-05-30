# Comments and worklogs

Both require the **detail pane** open (`Enter` on a row).

## Comments (`c`)

| Key | Action |
|-----|--------|
| `c` | Start comment input (footer) |
| Type text | Markdown → ADF on submit |
| `@` | Open assignable-user picker for this issue |
| `Enter` | Post comment |
| `Esc` | Cancel |

### @mention workflow

```text
Enter              # open detail
c                  # comment mode
Fixing @           # type @ → picker opens
j j                # highlight user
Enter              # insert @Display Name (stored as account id)
looks good         # rest of comment
Enter              # submit
```

Picker keys: `j`/`k`, `Enter` to insert, `Esc` closes picker only (keeps comment text).

### Markdown supported

- Headings `#` … `######`  
- `-` bullets, `1.` ordered lists  
- `- [ ]` / `- [x]` task lists  
- `**bold**`, `*italic*`, `` `code` ``, links, blockquotes, `---` rules  

## Worklogs (`w`)

| Key | Action |
|-----|--------|
| `w` | Start worklog input |
| `30m`, `1h`, `2d 4h` | Jira time spent format |
| `Enter` | Submit |
| `Esc` | Cancel |

### Examples

```text
w
30m
Enter

w
1h 15m
Enter
```

Invalid formats show a footer error from Jira.

## Related

- [detail-pane.md](detail-pane.md)
- [editing-fields.md](editing-fields.md) — description also uses `@`
