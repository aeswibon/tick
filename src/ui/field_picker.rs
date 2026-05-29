use ratatui::{
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

pub fn draw_field_picker(
    f: &mut Frame,
    title: &str,
    heading: &str,
    options: &[(String, String)],
    selected: usize,
    theme: &crate::theme::Theme,
    area: Rect,
) {
    let height = (options.len() as u16).saturating_add(6);
    let popup = centered_rect(58, height.min(area.height.saturating_sub(2)), area);
    f.render_widget(Clear, popup);

    let dl = theme.detail_label;
    let dv = theme.detail_value;

    let mut lines = vec![
        Line::from(Span::styled(
            format!(" {heading}"),
            Style::default().fg(dl).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    for (i, (_, label)) in options.iter().enumerate() {
        let is_sel = i == selected;
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
        let marker = if is_sel { "›" } else { " " };
        lines.push(Line::from(vec![
            Span::raw(format!(" {marker} ")),
            Span::styled(format!("{}. ", i + 1), num_style),
            Span::styled(label.as_str(), name_style),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  j/k move  Enter confirm  1-9  Esc cancel",
        Style::default().fg(theme.border),
    )));

    let popup_widget = Paragraph::new(Text::from(lines))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" {title} "))
                .border_style(Style::default().fg(theme.detail_border)),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(popup_widget, popup);
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
