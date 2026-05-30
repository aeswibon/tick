# Issue links & subtasks

View and add Jira issue links without opening the browser.

## Links tab

Open a ticket (`Enter`), then **`l`** until **Links** is active (or cycle with **`h`** / **`l`**).

The tab shows:

- **Issue links** — type, direction, key, status, summary
- **Subtasks** — child issues under the parent

Links and subtasks load when you open detail or change selection with detail open.

## Add a link (`I`)

With detail open (Links tab recommended):

1. Press **`I`** — pick link type (Relates, Blocks, Is blocked by, Epic)
2. **`Enter`** — type target issue key in the footer (e.g. `HIN-123`)
3. **`Enter`** again to create the link

| Type | Meaning |
|------|---------|
| Relates to | Generic relationship |
| Blocks | Current issue blocks the target |
| Is blocked by | Current issue is blocked by the target |
| Epic | Epic–story link (Jira `Epic-Story Link` type) |

Errors from Jira appear in the footer. On success, links refresh and views reload.

## Related

- [editing-fields.md](editing-fields.md) — field edits, watch/unwatch
- [create-duplicate-templates.md](create-duplicate-templates.md) — duplicate adds a Cloners link after create
