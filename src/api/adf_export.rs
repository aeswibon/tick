//! Convert Jira ADF documents to markdown for editing in the TUI.

use serde_json::Value;

/// Best-effort ADF → markdown for the description/comment editor buffer.
pub fn to_markdown(doc: &Value) -> String {
    let Some(content) = doc.get("content").and_then(|c| c.as_array()) else {
        return String::new();
    };
    let mut out = String::new();
    for (i, block) in content.iter().enumerate() {
        if i > 0 {
            let needs_gap = !out.ends_with("\n\n") && !block_gap_before(block);
            if needs_gap {
                out.push_str("\n\n");
            }
        }
        out.push_str(&block_to_md(block));
    }
    out.trim_end().to_string()
}

fn block_gap_before(block: &Value) -> bool {
    matches!(
        block.get("type").and_then(|t| t.as_str()),
        Some("paragraph") | Some("heading")
    )
}

fn block_to_md(node: &Value) -> String {
    match node.get("type").and_then(|t| t.as_str()) {
        Some("paragraph") => {
            let line = inline_children_to_md(node.get("content"));
            if line.is_empty() {
                String::new()
            } else {
                format!("{line}\n")
            }
        }
        Some("heading") => {
            let level = node
                .get("attrs")
                .and_then(|a| a.get("level"))
                .and_then(|l| l.as_u64())
                .unwrap_or(1)
                .clamp(1, 6);
            format!(
                "{} {}\n",
                "#".repeat(level as usize),
                inline_children_to_md(node.get("content"))
            )
        }
        Some("bulletList") => list_to_md(node, "- "),
        Some("orderedList") => {
            let mut s = String::new();
            if let Some(items) = node.get("content").and_then(|c| c.as_array()) {
                for (i, item) in items.iter().enumerate() {
                    s.push_str(&list_item_to_md(item, &format!("{}. ", i + 1)));
                }
            }
            s
        }
        Some("codeBlock") => {
            let lang = node
                .get("attrs")
                .and_then(|a| a.get("language"))
                .and_then(|l| l.as_str())
                .unwrap_or("");
            let code = node
                .get("content")
                .and_then(|c| c.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|n| n.get("text").and_then(|t| t.as_str()))
                        .collect::<Vec<_>>()
                        .join("")
                })
                .unwrap_or_default();
            format!("```{lang}\n{code}\n```\n")
        }
        Some("blockquote") => {
            let inner = children_to_md(node.get("content"));
            inner
                .lines()
                .map(|l| {
                    if l.is_empty() {
                        ">".to_string()
                    } else {
                        format!("> {l}")
                    }
                })
                .collect::<Vec<_>>()
                .join("\n")
                + "\n"
        }
        Some("rule") => "---\n".to_string(),
        Some("panel") => children_to_md(node.get("content")),
        Some("table") => table_to_md(node),
        Some("mediaSingle") | Some("mediaGroup") => media_container_to_md(node),
        Some("expand") | Some("nestedExpand") => {
            let title = node
                .get("attrs")
                .and_then(|a| a.get("title"))
                .and_then(|t| t.as_str())
                .unwrap_or("Details");
            format!("**{title}**\n\n{}", children_to_md(node.get("content")))
        }
        Some("taskList") => task_list_to_md(node),
        Some("decisionList") => decision_list_to_md(node),
        Some("status") => status_to_md(node),
        _ => unknown_block_to_md(node),
    }
}

fn unknown_block_to_md(node: &Value) -> String {
    serde_json::to_string(node)
        .map(|json| format!("```adf-json\n{json}\n```\n"))
        .unwrap_or_default()
}

fn task_list_to_md(node: &Value) -> String {
    let mut s = String::new();
    if let Some(items) = node.get("content").and_then(|c| c.as_array()) {
        for item in items {
            if item.get("type").and_then(|t| t.as_str()) == Some("taskItem") {
                s.push_str(&task_item_to_md(item));
            }
        }
    }
    s
}

fn task_item_to_md(item: &Value) -> String {
    let done = item
        .get("attrs")
        .and_then(|a| a.get("state"))
        .and_then(|s| s.as_str())
        .is_some_and(|s| s.eq_ignore_ascii_case("DONE") || s.eq_ignore_ascii_case("COMPLETE"));
    let mark = if done { "x" } else { " " };
    let text = task_item_text(item);
    format!("- [{mark}] {text}\n")
}

fn task_item_text(item: &Value) -> String {
    item.get("content")
        .and_then(|c| c.as_array())
        .map(|arr| {
            arr.iter()
                .map(|n| block_to_md(n).trim().replace('\n', " "))
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default()
}

fn decision_list_to_md(node: &Value) -> String {
    let mut s = String::new();
    if let Some(items) = node.get("content").and_then(|c| c.as_array()) {
        for item in items {
            if item.get("type").and_then(|t| t.as_str()) == Some("decisionItem") {
                let text = inline_children_to_md(item.get("content"));
                s.push_str(&format!("- ◆ {text}\n"));
            }
        }
    }
    s
}

fn status_to_md(node: &Value) -> String {
    let text = node
        .get("attrs")
        .and_then(|a| a.get("text"))
        .and_then(|t| t.as_str())
        .unwrap_or("status");
    format!("[status: {text}]\n")
}

fn children_to_md(content: Option<&Value>) -> String {
    let Some(arr) = content.and_then(|c| c.as_array()) else {
        return String::new();
    };
    let mut s = String::new();
    for (i, child) in arr.iter().enumerate() {
        if i > 0 && !s.ends_with('\n') {
            s.push('\n');
        }
        s.push_str(&block_to_md(child));
    }
    s
}

fn list_to_md(node: &Value, prefix: &str) -> String {
    let mut s = String::new();
    if let Some(items) = node.get("content").and_then(|c| c.as_array()) {
        for item in items {
            s.push_str(&list_item_to_md(item, prefix));
        }
    }
    s
}

fn list_item_to_md(item: &Value, prefix: &str) -> String {
    let Some(content) = item.get("content").and_then(|c| c.as_array()) else {
        return String::new();
    };
    let mut lines = Vec::new();
    for block in content {
        let text = block_to_md(block).trim_end().to_string();
        for (i, line) in text.lines().enumerate() {
            if i == 0 {
                lines.push(format!("{prefix}{line}"));
            } else {
                lines.push(format!("  {line}"));
            }
        }
    }
    if lines.is_empty() {
        format!("{prefix}\n")
    } else {
        lines.join("\n") + "\n"
    }
}

fn table_to_md(node: &Value) -> String {
    let Some(rows) = node.get("content").and_then(|c| c.as_array()) else {
        return String::new();
    };
    let mut md_rows: Vec<Vec<String>> = Vec::new();
    for row in rows {
        if row.get("type").and_then(|t| t.as_str()) != Some("tableRow") {
            continue;
        }
        let cells: Vec<String> = row
            .get("content")
            .and_then(|c| c.as_array())
            .map(|cells| {
                cells
                    .iter()
                    .map(|cell| cell_text(cell).replace('|', "\\|"))
                    .collect()
            })
            .unwrap_or_default();
        if !cells.is_empty() {
            md_rows.push(cells);
        }
    }
    if md_rows.is_empty() {
        return String::new();
    }
    let cols = md_rows.iter().map(|r| r.len()).max().unwrap_or(0);
    let mut out = String::new();
    for (i, row) in md_rows.iter().enumerate() {
        let mut padded = row.clone();
        while padded.len() < cols {
            padded.push(String::new());
        }
        out.push('|');
        for c in &padded {
            out.push(' ');
            out.push_str(c);
            out.push_str(" |");
        }
        out.push('\n');
        if i == 0 {
            out.push('|');
            for _ in 0..cols {
                out.push_str(" --- |");
            }
            out.push('\n');
        }
    }
    out
}

fn cell_text(cell: &Value) -> String {
    inline_children_to_md(cell.get("content"))
        .trim()
        .to_string()
}

fn media_container_to_md(node: &Value) -> String {
    let mut parts = Vec::new();
    if let Some(children) = node.get("content").and_then(|c| c.as_array()) {
        for child in children {
            if child.get("type").and_then(|t| t.as_str()) == Some("media") {
                parts.push(media_to_md(child));
            }
        }
    }
    if parts.is_empty() {
        String::new()
    } else {
        parts.join("\n") + "\n"
    }
}

fn media_to_md(node: &Value) -> String {
    let attrs = node.get("attrs");
    let alt = attrs
        .and_then(|a| a.get("alt"))
        .and_then(|v| v.as_str())
        .or_else(|| {
            attrs
                .and_then(|a| a.get("filename"))
                .and_then(|v| v.as_str())
        })
        .unwrap_or("attachment");
    let url = attrs
        .and_then(|a| a.get("url"))
        .and_then(|v| v.as_str())
        .or_else(|| attrs.and_then(|a| a.get("src")).and_then(|v| v.as_str()));
    match url {
        Some(href) if !href.is_empty() => format!("![{alt}]({href})"),
        _ => format!("![{alt}]"),
    }
}

fn inline_children_to_md(content: Option<&Value>) -> String {
    let Some(nodes) = content.and_then(|c| c.as_array()) else {
        return String::new();
    };
    nodes.iter().map(inline_to_md).collect()
}

fn inline_to_md(node: &Value) -> String {
    match node.get("type").and_then(|t| t.as_str()) {
        Some("text") => {
            let text = node.get("text").and_then(|t| t.as_str()).unwrap_or("");
            apply_marks(
                text,
                node.get("marks")
                    .and_then(|m| m.as_array())
                    .map(|a| a.as_slice()),
            )
        }
        Some("mention") => node
            .get("attrs")
            .and_then(|a| a.get("text"))
            .and_then(|t| t.as_str())
            .unwrap_or("@user")
            .to_string(),
        Some("hardBreak") => "\n".to_string(),
        Some("emoji") => node
            .get("attrs")
            .and_then(|a| a.get("text"))
            .and_then(|t| t.as_str())
            .map(String::from)
            .or_else(|| node.get("text").and_then(|t| t.as_str()).map(String::from))
            .or_else(|| {
                node.get("attrs")
                    .and_then(|a| a.get("shortName"))
                    .and_then(|s| s.as_str())
                    .map(|s| format!(":{s}:"))
            })
            .unwrap_or_default(),
        Some("status") => {
            let text = node
                .get("attrs")
                .and_then(|a| a.get("text"))
                .and_then(|t| t.as_str())
                .unwrap_or("status");
            format!("[status: {text}]")
        }
        Some("inlineCard") => node
            .get("attrs")
            .and_then(|a| a.get("url"))
            .and_then(|u| u.as_str())
            .map(|u| format!("<{u}>"))
            .unwrap_or_default(),
        _ => String::new(),
    }
}

fn apply_marks(text: &str, marks: Option<&[Value]>) -> String {
    let Some(marks) = marks else {
        return text.to_string();
    };
    let mut s = text.to_string();
    for mark in marks {
        match mark.get("type").and_then(|t| t.as_str()) {
            Some("strong") => s = format!("**{s}**"),
            Some("em") => s = format!("*{s}*"),
            Some("code") => s = format!("`{s}`"),
            Some("strike") => s = format!("~~{s}~~"),
            Some("underline") => {}
            Some("link") => {
                if let Some(href) = mark
                    .get("attrs")
                    .and_then(|a| a.get("href"))
                    .and_then(|h| h.as_str())
                {
                    s = format!("[{s}]({href})");
                }
            }
            _ => {}
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paragraph_and_heading() {
        let doc = serde_json::json!({
            "type": "doc",
            "content": [
                { "type": "heading", "attrs": { "level": 2 }, "content": [
                    { "type": "text", "text": "Title" }
                ]},
                { "type": "paragraph", "content": [
                    { "type": "text", "text": "bold", "marks": [{ "type": "strong" }] }
                ]}
            ]
        });
        let md = to_markdown(&doc);
        assert!(md.contains("## Title"));
        assert!(md.contains("**bold**"));
    }

    #[test]
    fn task_list_and_adf_json_fence() {
        let doc = serde_json::json!({
            "type": "doc",
            "content": [{
                "type": "taskList",
                "content": [{
                    "type": "taskItem",
                    "attrs": { "state": "TODO" },
                    "content": [{
                        "type": "paragraph",
                        "content": [{ "type": "text", "text": "Ship it" }]
                    }]
                }]
            }]
        });
        let md = to_markdown(&doc);
        assert!(md.contains("- [ ] Ship it"));

        let layout = serde_json::json!({ "type": "layoutSection", "content": [] });
        let md2 = to_markdown(&serde_json::json!({
            "type": "doc",
            "content": [layout.clone()]
        }));
        assert!(md2.contains("```adf-json"));
        let back = crate::api::markdown::to_adf(&md2, &[]);
        assert_eq!(back["content"][0], layout);
    }

    #[test]
    fn mention_and_list() {
        let doc = serde_json::json!({
            "type": "doc",
            "content": [{
                "type": "bulletList",
                "content": [{
                    "type": "listItem",
                    "content": [{
                        "type": "paragraph",
                        "content": [
                            { "type": "mention", "attrs": { "text": "@Ada" } }
                        ]
                    }]
                }]
            }]
        });
        let md = to_markdown(&doc);
        assert!(md.contains("- @Ada"));
    }
}
