use ratatui::{
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::app::App;

pub fn draw_site_errors(f: &mut Frame, app: &App, area: Rect) {
    let lines = &app.status.site_warnings;
    if lines.is_empty() {
        return;
    }

    let height = (lines.len() as u16 + 6).min(area.height.saturating_sub(2));
    let popup = centered_rect(70, height, area);
    f.render_widget(Clear, popup);

    let visible = visible_height(popup.height);
    let scroll = app
        .site_error_scroll
        .min(lines.len().saturating_sub(visible.max(1)));

    let mut text_lines = vec![
        Line::from(Span::styled(
            format!(" Site errors ({})", lines.len()),
            Style::default()
                .fg(app.theme.detail_label)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    for msg in lines.iter().skip(scroll) {
        if text_lines.len() >= visible + 2 {
            break;
        }
        text_lines.push(Line::from(vec![
            Span::styled(" • ", Style::default().fg(app.theme.priority_p1)),
            Span::styled(msg.as_str(), Style::default().fg(app.theme.detail_value)),
        ]));
    }

    text_lines.push(Line::from(""));
    text_lines.push(Line::from(Span::styled(
        " j/k scroll  Esc close",
        Style::default().fg(app.theme.border),
    )));

    let widget = Paragraph::new(Text::from(text_lines))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Site failures ")
                .border_style(Style::default().fg(app.theme.detail_border)),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(widget, popup);
}

fn visible_height(popup_height: u16) -> usize {
    popup_height.saturating_sub(4) as usize
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
