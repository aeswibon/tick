use crate::template_export_flow::{TemplateExportStep, TemplateExportSession};
use ratatui::{
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

pub fn draw_template_export(f: &mut Frame, session: &TemplateExportSession, theme: &crate::theme::Theme, area: Rect) {
    let (title, subtitle, show_clear) = match session.step {
        TemplateExportStep::IncludeFields => (
            "Export template — fields to save",
            "Space toggles include · Enter continues",
            false,
        ),
        TemplateExportStep::ClearValues => (
            "Export template — clear values",
            "Included fields only · Space = leave empty when creating (N)",
            true,
        ),
        TemplateExportStep::Name => return,
    };

    let visible: Vec<_> = session
        .rows
        .iter()
        .enumerate()
        .filter(|(_, r)| show_clear && r.include || !show_clear)
        .collect();

    let height = (visible.len() as u16).saturating_add(9).min(area.height.saturating_sub(2));
    let popup = centered_rect(62, height, area);
    f.render_widget(Clear, popup);

    let dl = theme.detail_label;
    let dv = theme.detail_value;
    let mut lines = vec![
        Line::from(Span::styled(
            format!(" From {}", session.source_key),
            Style::default().fg(theme.border),
        )),
        Line::from(Span::styled(
            format!(" {subtitle}"),
            Style::default().fg(theme.border),
        )),
        Line::from(""),
    ];

    for (vis_idx, (row_idx, row)) in visible.iter().enumerate() {
        let is_sel = *row_idx == session.selected;
        let marker = if is_sel { "›" } else { " " };

        let (box_ch, state_label) = if session.step == TemplateExportStep::IncludeFields {
            if row.include {
                ("☑", "save")
            } else {
                ("☐", "skip")
            }
        } else if row.clear_value {
            ("☐", "empty at create")
        } else {
            ("☑", "keep value")
        };

        let num_style = if is_sel {
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.priority_p1)
        };
        let name_style = if is_sel {
            Style::default().fg(dv).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(dv)
        };

        lines.push(Line::from(vec![
            Span::raw(format!(" {marker} ")),
            Span::styled(format!("{}. ", vis_idx + 1), num_style),
            Span::styled(format!("{box_ch} "), Style::default().fg(dl)),
            Span::styled(format!("{:<16}", row.label), name_style),
            Span::styled(
                format!(" [{state_label}] "),
                Style::default().fg(theme.border),
            ),
            Span::styled(row.preview.as_str(), Style::default().fg(theme.border)),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  j/k move  Space toggle  Enter next  Esc cancel",
        Style::default().fg(theme.border),
    )));

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {title} "))
        .border_style(Style::default().fg(theme.detail_border));

    f.render_widget(
        Paragraph::new(Text::from(lines))
            .block(block)
            .wrap(Wrap { trim: false }),
        popup,
    );
}

fn centered_rect(percent_x: u16, height: u16, r: Rect) -> Rect {
    let vert = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            Constraint::Length((r.height.saturating_sub(height)) / 2),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .split(r);

    ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([
            Constraint::Length((r.width * (100 - percent_x)) / 200),
            Constraint::Min(1),
            Constraint::Length((r.width * (100 - percent_x)) / 200),
        ])
        .split(vert[1])[1]
}
