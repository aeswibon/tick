use crate::app::App;
use crate::ui::markdown_preview::{markdown_preview_lines, preview_header_line};
use ratatui::{
    layout::{Constraint, Rect},
    style::Style,
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

pub fn draw_create_description_preview(f: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(70, 80, area);
    f.render_widget(Clear, popup);

    let dl = app.theme.detail_label;
    let border = Style::default().fg(app.theme.border);

    let mut lines = vec![
        preview_header_line(" Description preview ", Style::default().fg(dl)),
        Line::from(""),
    ];
    lines.extend(markdown_preview_lines(
        &app.input_buffer,
        &app.input_mentions,
        "No description",
        border,
    ));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Ctrl+P edit  Enter submit  Esc back to edit",
        border,
    )));

    let widget = Paragraph::new(Text::from(lines))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Preview ")
                .border_style(Style::default().fg(app.theme.detail_border)),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(widget, popup);
}

fn centered_rect(percent_x: u16, percent_height: u16, r: Rect) -> Rect {
    let height = (r.height * percent_height / 100).max(8);
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
