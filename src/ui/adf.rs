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

fn parse_inline(node: &serde_json::Value) -> Vec<Span<'static>> {
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
            _ => {}
        }
    }
    lines
}

pub fn render_doc(doc: &serde_json::Value) -> Vec<Line<'static>> {
    let content = match doc.get("content").and_then(|c| c.as_array()) {
        Some(c) => c,
        None => return vec![Line::from(Span::raw(""))],
    };
    render_content(content, 0)
}
