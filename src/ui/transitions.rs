use ratatui::{
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};
use crate::app::App;

pub fn draw_transitions(f: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(50, app.transition_options.len() as u16 + 4, area);
    f.render_widget(Clear, popup);

    let dl = app.theme.detail_label;
    let dv = app.theme.detail_value;

    let mut lines = vec![
        Line::from(Span::styled(" Select transition", Style::default().fg(dl).add_modifier(Modifier::BOLD))),
        Line::from(""),
    ];

    for (i, (_, name)) in app.transition_options.iter().enumerate() {
        lines.push(Line::from(vec![
            Span::styled(format!("  {}. ", i + 1), Style::default().fg(app.theme.priority_p1)),
            Span::styled(name, Style::default().fg(dv)),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("  Esc to cancel", Style::default().fg(app.theme.border))));

    let popup_widget = Paragraph::new(Text::from(lines))
        .block(Block::default()
            .borders(Borders::ALL)
            .title(" Transition ")
            .border_style(Style::default().fg(app.theme.detail_border)))
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
