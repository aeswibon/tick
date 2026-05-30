use crate::app::App;
use ratatui::{
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

pub fn draw_editable_field_picker(f: &mut Frame, app: &App, area: Rect) {
    let fields = &app.config.detail.editable_fields;
    let height = fields.len() as u16 + 4;
    let popup = centered_rect(55, height, area);
    f.render_widget(Clear, popup);

    let dl = app.theme.detail_label;
    let dv = app.theme.detail_value;

    let mut lines = vec![
        Line::from(Span::styled(
            " Edit custom field",
            Style::default().fg(dl).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    for (i, field) in fields.iter().enumerate() {
        let selected = i == app.editable_field_picker_selected;
        let marker = if selected { "›" } else { " " };
        let style = if selected {
            Style::default().fg(dv).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(dv)
        };
        lines.push(Line::from(vec![
            Span::raw(format!(" {marker} ")),
            Span::styled(
                format!("{}. ", i + 1),
                Style::default().fg(app.theme.accent),
            ),
            Span::styled(field.display_label(), style),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  j/k move  Enter confirm  1-9  Esc cancel",
        Style::default().fg(app.theme.border),
    )));

    let widget = Paragraph::new(Text::from(lines))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Custom field ")
                .border_style(Style::default().fg(app.theme.detail_border)),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(widget, popup);
}

pub fn draw_custom_field_select(f: &mut Frame, app: &App, area: Rect) {
    let height = app.custom_field_select_options.len() as u16 + 4;
    let popup = centered_rect(50, height, area);
    f.render_widget(Clear, popup);

    let dl = app.theme.detail_label;
    let dv = app.theme.detail_value;
    let title = app
        .custom_field_editing
        .as_ref()
        .map(|f| f.display_label())
        .unwrap_or_else(|| "Custom field".into());

    let mut lines = vec![
        Line::from(Span::styled(
            format!(" Select {title}"),
            Style::default().fg(dl).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    for (i, option) in app.custom_field_select_options.iter().enumerate() {
        let selected = i == app.custom_field_select_selected;
        let marker = if selected { "›" } else { " " };
        let style = if selected {
            Style::default().fg(dv).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(dv)
        };
        lines.push(Line::from(vec![
            Span::raw(format!(" {marker} ")),
            Span::styled(
                format!("{}. ", i + 1),
                Style::default().fg(app.theme.accent),
            ),
            Span::styled(option.as_str(), style),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  j/k move  Enter confirm  1-9  Esc cancel",
        Style::default().fg(app.theme.border),
    )));

    let widget = Paragraph::new(Text::from(lines))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" {title} "))
                .border_style(Style::default().fg(app.theme.detail_border)),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(widget, popup);
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
