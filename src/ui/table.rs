use ratatui::{
    layout::Constraint,
    style::Style,
    text::Span,
    widgets::{Cell, Row, Table},
    Frame,
};

use crate::app::App;
use crate::columns::Column;
use crate::ticket_lock::read_tickets;

pub fn draw_table(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    let viewport_rows = area.height.saturating_sub(1).max(1) as usize;
    app.set_table_viewport(viewport_rows);
    let tickets = read_tickets(&app.tickets);
    let indices = app.visible_indices();
    let columns = &app.columns;
    let selected_row = app.selected_viewport_row();

    let rows: Vec<Row> = indices
        .iter()
        .enumerate()
        .map(|(pos, &idx)| {
            let t = &tickets[idx];
            let is_selected = pos == selected_row;
            let row_style = if is_selected {
                app.theme.selected_style()
            } else if idx % 2 == 1 {
                Style::default()
                    .fg(app.theme.row_fg)
                    .bg(app.theme.row_alt_bg)
            } else {
                Style::default().fg(app.theme.row_fg)
            };

            let cells: Vec<Cell> = columns.iter().map(|col| col.cell(t, &app.theme)).collect();

            Row::new(cells).style(row_style)
        })
        .collect();

    let widths: Vec<Constraint> = columns
        .iter()
        .map(|col| {
            let w = col.width_hint();
            if matches!(
                col,
                Column::Status | Column::Assignee | Column::Reporter | Column::Summary
            ) {
                Constraint::Min(w)
            } else {
                Constraint::Length(w)
            }
        })
        .collect();

    let header_style = app.theme.header_style();
    let header_cells: Vec<Cell> = columns
        .iter()
        .map(|col| Cell::from(Span::styled(col.header(), header_style)))
        .collect();

    let table = Table::new(rows, widths).header(Row::new(header_cells));

    f.render_widget(table, area);
}
