use crate::app::App;
use ratatui::{
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

pub fn draw_mentions(f: &mut Frame, app: &App, area: Rect) {
    let count = app.mention_options.len().max(1) as u16;
    let popup = centered_rect(55, count + 5, area);
    f.render_widget(Clear, popup);

    let dl = app.theme.detail_label;
    let dv = app.theme.detail_value;

    let mut lines = vec![
        Line::from(Span::styled(
            " Tag user (@mention)",
            Style::default().fg(dl).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    if app.mention_options.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No users found — keep typing after @",
            Style::default().fg(app.theme.border),
        )));
    } else {
        for (i, (_, name)) in app.mention_options.iter().enumerate() {
            let selected = i == app.mention_selected;
            let marker = if selected { "›" } else { " " };
            let style = if selected {
                Style::default()
                    .fg(app.theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(dv)
            };
            lines.push(Line::from(vec![
                Span::raw(format!(" {marker} ")),
                Span::styled(format!("@{}", name), style),
            ]));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  j/k move  Enter pick  R add users  Esc cancel",
        Style::default().fg(app.theme.border),
    )));

    let popup_widget = Paragraph::new(Text::from(lines))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Mention ")
                .border_style(Style::default().fg(app.theme.detail_border)),
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
