use ratatui::{
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

pub fn draw_add_link_picker(
    f: &mut Frame,
    options: &[(String, String)],
    selected: usize,
    theme: &crate::theme::Theme,
    area: Rect,
) {
    let height = (options.len() as u16).saturating_add(8);
    let popup = centered_rect(50, height.min(area.height.saturating_sub(2)), area);
    f.render_widget(Clear, popup);

    let dl = theme.detail_label;
    let dv = theme.detail_value;
    let mut lines = vec![
        Line::from(Span::styled(
            " Link type",
            Style::default().fg(dl).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            " Enter to choose · Esc cancel",
            Style::default().fg(theme.border),
        )),
        Line::from(""),
    ];

    for (i, (_, label)) in options.iter().enumerate() {
        let is_sel = i == selected;
        let marker = if is_sel { "›" } else { " " };
        let style = if is_sel {
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(dv)
        };
        lines.push(Line::from(vec![
            Span::styled(format!(" {marker} "), style),
            Span::styled(label.as_str(), style),
        ]));
    }

    let block = Paragraph::new(Text::from(lines))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Add issue link "),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(block, popup);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            Constraint::Length((r.height * (100 - percent_y)) / 200),
            Constraint::Min(1),
            Constraint::Length((r.height * (100 - percent_y)) / 200),
        ])
        .split(r);

    ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([
            Constraint::Length((r.width * (100 - percent_x)) / 200),
            Constraint::Min(1),
            Constraint::Length((r.width * (100 - percent_x)) / 200),
        ])
        .split(popup_layout[1])[1]
}
