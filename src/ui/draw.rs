use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::App;
use crate::view_mode::ViewMode;

pub fn render(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(f.area());

    let (header_area, tab_area, main_area, _gap, footer_area) =
        (chunks[0], chunks[1], chunks[2], chunks[3], chunks[4]);

    render_header(f, app, header_area);
    render_tabs(f, app, tab_area);

    if app.detail_open {
        super::detail::draw_detail(f, app, main_area);
    } else {
        super::table::draw_table(f, app, main_area);
    }

    if app.show_help {
        super::help::draw_help(f, app, f.area());
    }
    if app.showing_transitions {
        super::transitions::draw_transitions(f, app, f.area());
    }
    if app.showing_priorities {
        super::priorities::draw_priorities(f, app, f.area());
    }
    if app.show_site_errors {
        super::errors::draw_site_errors(f, app, f.area());
    }

    render_footer(f, app, footer_area);
}

fn render_header(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let count = app.filtered_count();
    let sites = app.sites_str();
    let mins = app.last_refresh.elapsed().as_secs() / 60;
    let mode = if app.loading {
        "loading"
    } else if !app.live_data {
        "cached"
    } else {
        "live"
    };
    let right_text = format!(" {count} tickets | {mode} | refresh {mins}m ago");

    let header_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(1), Constraint::Length(7)])
        .split(area);

    f.render_widget(
        Paragraph::new(Line::from(Span::raw(format!(" {sites} | {right_text}"))))
            .style(Style::default().bg(app.theme.header_bg).fg(app.theme.fg)),
        header_chunks[0],
    );
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "[tick]",
            Style::default().fg(app.theme.tick_fg),
        )))
        .style(Style::default().bg(app.theme.header_bg)),
        header_chunks[1],
    );
}

fn render_tabs(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let tabs = ViewMode::all();
    let mut tab_spans = Vec::new();
    for tab in tabs {
        let is_active = tab == app.active_view;
        let label = tab.label();
        let style = if is_active {
            Style::default()
                .fg(app.theme.accent)
                .add_modifier(ratatui::style::Modifier::BOLD)
        } else {
            Style::default().fg(app.theme.border)
        };
        tab_spans.push(Span::styled(
            if is_active {
                format!(" [{label}]")
            } else {
                format!("  {label}")
            },
            style,
        ));
    }
    f.render_widget(Paragraph::new(Line::from(tab_spans)), area);
}

fn render_footer(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let (footer_text, fg_color) = if app.filtering {
        (format!(" Filter: {}_", app.filter), app.theme.accent)
    } else if app.input_mode == crate::app::InputMode::Comment {
        (format!(" Comment: {}_", app.input_buffer), app.theme.accent)
    } else if app.input_mode == crate::app::InputMode::Worklog {
        (
            format!(" Worklog (e.g. 30m): {}_", app.input_buffer),
            app.theme.accent,
        )
    } else if app.input_mode == crate::app::InputMode::EditSummary {
        (format!(" Summary: {}_", app.input_buffer), app.theme.accent)
    } else if app.loading {
        (" Loading...".into(), app.theme.loading_fg)
    } else if let Some(ref err) = app.status.action_error {
        (format!(" Error: {err}"), app.theme.error_fg)
    } else if app.status.has_warnings() {
        (
            format!(
                " {} site error(s) — press ! for details",
                app.status.site_warnings.len()
            ),
            app.theme.loading_fg,
        )
    } else {
        let mut left = " ? help  / filter  ! errors  j/k  s sort  y copy  t trans  [ ] scroll  ←/→ view  1-4 tabs  q quit".to_string();
        if app.detail_open {
            left.push_str("  S/P summary/priority  h/l tabs");
        }
        let total = app.filtered_count();
        let row = if total == 0 { 0 } else { app.selected + 1 };
        let right = format!(
            " {} | {row}/{total} | Sort: {} | {} tickets",
            app.active_view.label(),
            app.sort_mode.label(),
            total
        );
        (format!("{left:<72}{right}"), app.theme.footer_fg)
    };

    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            footer_text,
            Style::default().fg(fg_color),
        )))
        .style(Style::default().bg(app.theme.footer_bg)),
        area,
    );
}
