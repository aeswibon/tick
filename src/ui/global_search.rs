use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

use crate::app::App;

pub fn draw_global_search(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Search cached views (Ctrl+g) ");

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1)])
        .split(inner);

    let items: Vec<ListItem> = app
        .global_search_hits
        .iter()
        .enumerate()
        .map(|(i, hit)| {
            let line = format!(
                "{}  {}  {} — {}",
                hit.view_label, hit.ticket.site, hit.ticket.key, hit.ticket.summary
            );
            let style = if i == app.global_search_selected {
                Style::default()
                    .fg(app.theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(app.theme.fg)
            };
            ListItem::new(Line::from(Span::styled(line, style)))
        })
        .collect();

    let hint = if app.global_search_hits.is_empty() {
        "No matches in cached views"
    } else {
        "j/k move · Enter open · Esc cancel"
    };
    f.render_widget(
        List::new(items).block(Block::default().title(hint)),
        chunks[0],
    );
    f.render_widget(
        ratatui::widgets::Paragraph::new(format!(" Query: {} ", app.input_buffer)),
        chunks[1],
    );
}
