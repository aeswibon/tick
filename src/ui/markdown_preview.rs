//! Render markdown (as ADF) for live preview overlays.

use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

/// Convert markdown + mentions to ratatui lines (same pipeline as saved descriptions).
pub fn markdown_preview_lines(
    text: &str,
    mentions: &[(String, String)],
    empty_label: &str,
    border_style: Style,
) -> Vec<Line<'static>> {
    if text.trim().is_empty() {
        return vec![Line::from(Span::styled(
            format!("  {empty_label}"),
            border_style,
        ))];
    }
    let adf = crate::api::markdown::to_adf(text, mentions);
    let mut lines = crate::ui::adf::render_doc(&adf);
    if lines.is_empty() {
        lines.push(Line::from(Span::styled(
            format!("  {empty_label}"),
            border_style,
        )));
    }
    lines
}

pub fn preview_header_line(title: impl Into<String>, label_style: Style) -> Line<'static> {
    Line::from(Span::styled(
        title.into(),
        label_style.add_modifier(Modifier::BOLD),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_renders_non_empty_markdown() {
        let lines = markdown_preview_lines("# Title\n\nBody", &[], "(empty)", Style::default());
        assert!(!lines.is_empty());
        let joined: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.clone()))
            .collect();
        assert!(joined.contains("Title") || joined.contains("Body"));
    }

    #[test]
    fn preview_empty_shows_placeholder() {
        let lines = markdown_preview_lines("  \n", &[], "(empty)", Style::default());
        assert!(lines[0].spans[0].content.contains("(empty)"));
    }
}
