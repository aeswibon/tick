//! Render Jira ADF JSON in the detail pane. Request bodies use `api::adf`.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

fn color_hex(hex: u32) -> Color {
    Color::Rgb(
        ((hex >> 16) & 0xFF) as u8,
        ((hex >> 8) & 0xFF) as u8,
        (hex & 0xFF) as u8,
    )
}

fn mention_style() -> Style {
    Style::default()
        .fg(color_hex(0xF9E2AF))
        .add_modifier(Modifier::BOLD)
}

fn parse_inline(node: &serde_json::Value) -> Vec<Span<'static>> {
    match node.get("type").and_then(|t| t.as_str()) {
        Some("mention") => {
            let label = node
                .get("attrs")
                .and_then(|a| a.get("text"))
                .and_then(|t| t.as_str())
                .filter(|s| !s.is_empty())
                .or_else(|| {
                    node.get("attrs")
                        .and_then(|a| a.get("id"))
                        .and_then(|t| t.as_str())
                })
                .unwrap_or("@user")
                .to_string();
            return vec![Span::styled(label, mention_style())];
        }
        Some("hardBreak") => return vec![Span::raw("\n")],
        Some("emoji") => {
            let short = node
                .get("attrs")
                .and_then(|a| a.get("shortName"))
                .and_then(|s| s.as_str());
            let text = node
                .get("text")
                .and_then(|t| t.as_str())
                .map(String::from)
                .or_else(|| short.map(|s| format!(":{s}:")))
                .unwrap_or_default();
            return vec![Span::raw(text)];
        }
        Some("text") | None => {}
        _ => return vec![],
    }

    let text = node.get("text").and_then(|t| t.as_str()).unwrap_or("");
    let text = text.to_string();
    let mut style = Style::default();
    if let Some(marks) = node.get("marks").and_then(|m| m.as_array()) {
        for mark in marks {
            match mark.get("type").and_then(|t| t.as_str()) {
                Some("strong") => style = style.add_modifier(Modifier::BOLD),
                Some("em") => style = style.add_modifier(Modifier::ITALIC),
                Some("code") => style = style.fg(color_hex(0xE6DB74)).bg(color_hex(0x272822)),
                Some("strike") => style = style.add_modifier(Modifier::CROSSED_OUT),
                Some("underline") => style = style.add_modifier(Modifier::UNDERLINED),
                Some("link") => {
                    if let Some(href) = mark
                        .get("attrs")
                        .and_then(|a| a.get("href"))
                        .and_then(|h| h.as_str())
                    {
                        style = style.fg(color_hex(0x66D9EF));
                        return vec![Span::styled(format!("{} ({})", text, href), style)];
                    }
                }
                _ => {}
            }
        }
    }
    vec![Span::styled(text, style)]
}

fn render_content(content: &[serde_json::Value], indent: usize) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    for node in content {
        match node.get("type").and_then(|t| t.as_str()) {
            Some("paragraph") => {
                let mut spans: Vec<Span<'static>> = Vec::new();
                if let Some(inline) = node.get("content").and_then(|c| c.as_array()) {
                    for child in inline {
                        spans.extend(parse_inline(child));
                    }
                }
                let mut line = Line::from(spans);
                if indent > 0 {
                    let mut indented_spans: Vec<Span<'static>> =
                        vec![Span::raw(" ".repeat(indent * 2))];
                    indented_spans.append(&mut line.spans);
                    line = Line::from(indented_spans);
                }
                lines.push(line);
                lines.push(Line::from(""));
            }
            Some("heading") => {
                let level = node
                    .get("attrs")
                    .and_then(|a| a.get("level"))
                    .and_then(|l| l.as_u64())
                    .unwrap_or(1);
                let mut spans: Vec<Span<'static>> = vec![Span::styled(
                    format!("{} ", "#".repeat(level as usize)),
                    Style::default()
                        .fg(color_hex(0x89B4FA))
                        .add_modifier(Modifier::BOLD),
                )];
                if let Some(inline) = node.get("content").and_then(|c| c.as_array()) {
                    for child in inline {
                        spans.extend(parse_inline(child));
                    }
                }
                lines.push(Line::from(spans));
                lines.push(Line::from(""));
            }
            Some("bulletList") => {
                if let Some(items) = node.get("content").and_then(|c| c.as_array()) {
                    for item in items {
                        if let Some(item_content) = item.get("content").and_then(|c| c.as_array()) {
                            let rendered: Vec<Line<'static>> =
                                render_content(item_content, indent + 1);
                            if let Some(first) = rendered.first() {
                                let mut spans: Vec<Span<'static>> =
                                    vec![Span::raw(format!("{}• ", "  ".repeat(indent)))];
                                spans.extend(first.spans.clone());
                                lines.push(Line::from(spans));
                                for rest in rendered.iter().skip(1) {
                                    let mut rest_spans: Vec<Span<'static>> =
                                        vec![Span::raw(format!("{}  ", "  ".repeat(indent)))];
                                    rest_spans.extend(rest.spans.clone());
                                    lines.push(Line::from(rest_spans));
                                }
                            }
                        }
                    }
                }
            }
            Some("orderedList") => {
                if let Some(items) = node.get("content").and_then(|c| c.as_array()) {
                    for (i, item) in items.iter().enumerate() {
                        if let Some(item_content) = item.get("content").and_then(|c| c.as_array()) {
                            let rendered: Vec<Line<'static>> =
                                render_content(item_content, indent + 1);
                            if let Some(first) = rendered.first() {
                                let mut spans: Vec<Span<'static>> =
                                    vec![Span::raw(format!("{}{}. ", "  ".repeat(indent), i + 1))];
                                spans.extend(first.spans.clone());
                                lines.push(Line::from(spans));
                                for rest in rendered.iter().skip(1) {
                                    let mut rest_spans: Vec<Span<'static>> =
                                        vec![Span::raw(format!("{}  ", "  ".repeat(indent)))];
                                    rest_spans.extend(rest.spans.clone());
                                    lines.push(Line::from(rest_spans));
                                }
                            }
                        }
                    }
                }
            }
            Some("codeBlock") => {
                let lang = node
                    .get("attrs")
                    .and_then(|a| a.get("language"))
                    .and_then(|l| l.as_str())
                    .unwrap_or("");
                if !lang.is_empty() {
                    lines.push(Line::from(Span::styled(
                        format!("```{}", lang),
                        Style::default()
                            .fg(color_hex(0x66D9EF))
                            .add_modifier(Modifier::BOLD),
                    )));
                } else {
                    lines.push(Line::from(Span::styled(
                        "```",
                        Style::default()
                            .fg(color_hex(0x66D9EF))
                            .add_modifier(Modifier::BOLD),
                    )));
                }
                if let Some(code_content) = node.get("content").and_then(|c| c.as_array()) {
                    for child in code_content {
                        if let Some(text) = child.get("text").and_then(|t| t.as_str()) {
                            for line in text.lines() {
                                lines.push(Line::from(Span::styled(
                                    line.to_string(),
                                    Style::default()
                                        .fg(color_hex(0xE6DB74))
                                        .bg(color_hex(0x272822)),
                                )));
                            }
                        }
                    }
                }
                lines.push(Line::from(Span::styled(
                    "```",
                    Style::default()
                        .fg(color_hex(0x66D9EF))
                        .add_modifier(Modifier::BOLD),
                )));
                lines.push(Line::from(""));
            }
            Some("rule") => {
                lines.push(Line::from(Span::styled(
                    "─".repeat(40),
                    Style::default().fg(color_hex(0x45475A)),
                )));
                lines.push(Line::from(""));
            }
            Some("panel") => {
                if let Some(panel_content) = node.get("content").and_then(|c| c.as_array()) {
                    lines.extend(render_content(panel_content, indent));
                }
            }
            Some("blockquote") => {
                if let Some(quote_content) = node.get("content").and_then(|c| c.as_array()) {
                    for quote_line in render_content(quote_content, indent) {
                        let mut spans: Vec<Span<'static>> =
                            vec![Span::styled("│ ", Style::default().fg(color_hex(0x45475A)))];
                        spans.extend(quote_line.spans);
                        lines.push(Line::from(spans));
                    }
                }
            }
            Some("table") => {
                lines.extend(render_table(node, indent));
            }
            Some("mediaSingle") | Some("mediaGroup") => {
                if let Some(children) = node.get("content").and_then(|c| c.as_array()) {
                    for child in children {
                        if child.get("type").and_then(|t| t.as_str()) == Some("media") {
                            lines.push(render_media_line(child, indent));
                            lines.push(Line::from(""));
                        }
                    }
                }
            }
            Some("expand") | Some("nestedExpand") => {
                let title = node
                    .get("attrs")
                    .and_then(|a| a.get("title"))
                    .and_then(|t| t.as_str())
                    .unwrap_or("Details");
                lines.push(Line::from(Span::styled(
                    format!("▸ {title}"),
                    Style::default()
                        .fg(color_hex(0x89B4FA))
                        .add_modifier(Modifier::BOLD),
                )));
                if let Some(inner) = node.get("content").and_then(|c| c.as_array()) {
                    lines.extend(render_content(inner, indent + 1));
                }
            }
            Some(other) => {
                lines.push(Line::from(Span::styled(
                    format!("  [{other}]"),
                    Style::default().fg(color_hex(0x6C7086)),
                )));
            }
            None => {}
        }
    }
    lines
}

fn render_media_line(node: &serde_json::Value, indent: usize) -> Line<'static> {
    let pad = "  ".repeat(indent);
    let attrs = node.get("attrs");
    let name = attrs
        .and_then(|a| a.get("alt"))
        .or_else(|| attrs.and_then(|a| a.get("filename")))
        .and_then(|v| v.as_str())
        .unwrap_or("attachment");
    let url = attrs
        .and_then(|a| a.get("url"))
        .or_else(|| attrs.and_then(|a| a.get("src")))
        .and_then(|v| v.as_str());
    let body = match url {
        Some(href) if !href.is_empty() => format!("📎 {name} — {href}"),
        _ => format!("📎 {name}"),
    };
    Line::from(Span::styled(
        format!("{pad}{body}"),
        Style::default().fg(color_hex(0x94E2D5)),
    ))
}

fn render_table(node: &serde_json::Value, indent: usize) -> Vec<Line<'static>> {
    let pad = "  ".repeat(indent);
    let Some(rows) = node.get("content").and_then(|c| c.as_array()) else {
        return vec![];
    };
    let mut parsed: Vec<Vec<String>> = Vec::new();
    for row in rows {
        if row.get("type").and_then(|t| t.as_str()) != Some("tableRow") {
            continue;
        }
        let cells: Vec<String> = row
            .get("content")
            .and_then(|c| c.as_array())
            .map(|cells| cells.iter().map(table_cell_text).collect())
            .unwrap_or_default();
        if !cells.is_empty() {
            parsed.push(cells);
        }
    }
    if parsed.is_empty() {
        return vec![];
    }
    let cols = parsed.iter().map(|r| r.len()).max().unwrap_or(0);
    let mut lines = Vec::new();
    let border = Style::default().fg(color_hex(0x45475A));
    let cell_style = Style::default().fg(color_hex(0xCDD6F4));
    for (ri, row) in parsed.iter().enumerate() {
        let mut padded = row.clone();
        while padded.len() < cols {
            padded.push(String::new());
        }
        let mut spans: Vec<Span<'static>> = vec![Span::raw(pad.clone())];
        for (ci, cell) in padded.iter().enumerate() {
            if ci > 0 {
                spans.push(Span::styled(" │ ", border));
            }
            let display = if cell.len() > 24 {
                format!("{}…", &cell[..23])
            } else {
                cell.clone()
            };
            spans.push(Span::styled(display, cell_style));
        }
        lines.push(Line::from(spans));
        if ri == 0 {
            let rule: String = (0..cols)
                .map(|_| "────────")
                .collect::<Vec<_>>()
                .join("─┼─");
            lines.push(Line::from(vec![
                Span::raw(pad.clone()),
                Span::styled(rule, border),
            ]));
        }
    }
    lines.push(Line::from(""));
    lines
}

fn table_cell_text(cell: &serde_json::Value) -> String {
    let Some(content) = cell.get("content").and_then(|c| c.as_array()) else {
        return String::new();
    };
    let mut parts = Vec::new();
    for block in content {
        if let Some(inline) = block.get("content").and_then(|c| c.as_array()) {
            for child in inline {
                if let Some(t) = child.get("text").and_then(|x| x.as_str()) {
                    parts.push(t);
                } else if child.get("type").and_then(|t| t.as_str()) == Some("mention") {
                    if let Some(label) = child
                        .get("attrs")
                        .and_then(|a| a.get("text"))
                        .and_then(|t| t.as_str())
                    {
                        parts.push(label);
                    }
                }
            }
        }
    }
    parts.join(" ").trim().to_string()
}

pub fn render_doc(doc: &serde_json::Value) -> Vec<Line<'static>> {
    let content = match doc.get("content").and_then(|c| c.as_array()) {
        Some(c) => c,
        None => return vec![Line::from(Span::raw(""))],
    };
    render_content(content, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line_text(line: &Line) -> String {
        line.spans
            .iter()
            .map(|s| s.content.clone())
            .collect::<Vec<_>>()
            .join("")
    }

    #[test]
    fn renders_mention_node_with_label() {
        let doc = serde_json::json!({
            "type": "doc",
            "content": [{
                "type": "paragraph",
                "content": [
                    {"type": "text", "text": "ping "},
                    {"type": "mention", "attrs": {"id": "1", "text": "@Ada"}},
                ]
            }]
        });
        let lines = render_doc(&doc);
        let text = line_text(&lines[0]);
        assert!(text.contains("ping "));
        assert!(text.contains("@Ada"));
    }

    #[test]
    fn renders_table_rows() {
        let doc = serde_json::json!({
            "type": "doc",
            "content": [{
                "type": "table",
                "content": [{
                    "type": "tableRow",
                    "content": [
                        { "type": "tableHeader", "content": [{
                            "type": "paragraph",
                            "content": [{ "type": "text", "text": "A" }]
                        }]},
                        { "type": "tableHeader", "content": [{
                            "type": "paragraph",
                            "content": [{ "type": "text", "text": "B" }]
                        }]}
                    ]
                }]
            }]
        });
        let lines = render_doc(&doc);
        let joined = lines
            .iter()
            .map(|l| line_text(l))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(joined.contains('A'));
        assert!(joined.contains('B'));
        assert!(joined.contains('│'));
    }

    #[test]
    fn renders_hard_break_as_newline() {
        let doc = serde_json::json!({
            "type": "doc",
            "content": [{
                "type": "paragraph",
                "content": [
                    {"type": "text", "text": "a"},
                    {"type": "hardBreak"},
                    {"type": "text", "text": "b"},
                ]
            }]
        });
        let lines = render_doc(&doc);
        assert!(line_text(&lines[0]).contains('\n'));
    }
}
