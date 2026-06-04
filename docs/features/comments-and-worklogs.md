# Comments and worklogs

Both require the **detail pane** open (`Enter` on a row).

## Comments (`c`)

| Key | Action |
|-----|--------|
| `c` | Start comment input (footer) |
| Type text | Markdown → ADF on submit (stored locally until **Enter**) |
| `@` | Open assignable-user picker for this issue |
| `Shift+Enter` | New line in comment |
| `Ctrl+P` | Toggle live markdown preview overlay |
| `Ctrl+U` | Queue a file attachment (path prompt) |
| `←` / `→` | Move cursor in footer input |
| `Alt+←` / `Alt+→` | Move by word |
| `Ctrl+Backspace` | Delete word before cursor |
| `Delete` | Delete character after cursor |
| `Ctrl+A` / `Ctrl+E` | Start / end of footer buffer |
| `Ctrl+V` | Paste text from clipboard |
| `Ctrl+Shift+V` | Paste clipboard image as attachment (comment mode) |
| `Home` / `End` | Start / end of current footer line |
| `Enter` | Post comment to Jira |
| `Esc` | Cancel (or back from preview / attach path) |

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

### Typing performance

Comment text is held in the footer until you press **Enter** — nothing is sent to Jira while you type. The `@` mention picker only queries Jira when your cursor is in an active `@mention` (after `@` and before a space). Typing normal text does not hit the network.

### Multiline and paste

- Use **Shift+Enter** (or **Alt+Enter**) for a new line without submitting.
- Bracketed **paste** (terminal paste) inserts the full text at once; pasted newlines no longer advance to the next field.
- The footer grows to **3 lines** and wraps long lines while composing comments or descriptions.

### Markdown supported

- Headings `#` … `######`  
- `-` bullets, `1.` ordered lists  
- `- [ ]` / `- [x]` task lists  
- `**bold**`, `*italic*`, `` `code` ``, `[label](https://example.com)` links, blockquotes, `---` rules  
- Bare `http://` and `https://` URLs are auto-linked on submit  
- `![alt](https://example.com/image.png)` external images become inline ADF media  

### Attachments

- **Ctrl+U** while composing opens an attachment path prompt. Enter a local file path; it is queued and shown in the footer (and preview). Repeat to queue multiple files.
- On submit, **images** (`png`, `jpg`, `gif`, `webp`, etc.) upload via Jira media API and render inline in the comment. **Other files** upload as issue attachments with a linked filename in the comment body.
- **CLI:** `tick issue comment KEY --body "…" --attach /path/to/file.pdf` (repeat `--attach` for multiple files).

### Open links from Comments tab

When comments are loaded, attachment URLs and markdown links appear in a **Links in comments** list at the top of the tab.

| Key | Action |
|-----|--------|
| `j` / `k` | Select a link (when the list is shown) |
| `o` | Open selected link in the browser |

Table row selection still uses `j`/`k` when there are no openable links in comments.

### Footer vs normal mode keys

While the footer is active, **table navigation keys** (`j`/`k`, `1`–`6`, etc.) edit the prompt instead of moving the table. **`j`/`k`** still work inside the `@` mention picker. Use **Esc** to cancel footer input and return to normal navigation.

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
