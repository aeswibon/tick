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
    if app.showing_create_picker {
        super::create::draw_create_picker(f, app, f.area());
    }
    if let Some(ref session) = app.template_export {
        if session.step != crate::template_export_flow::TemplateExportStep::Name {
            super::template_export::draw_template_export(f, session, &app.theme, f.area());
        }
    }
    if let Some(ref session) = app.template_manage {
        super::template_manage::draw_template_manage(f, session, &app.theme, f.area());
    }
    if app.showing_transitions {
        super::transitions::draw_transitions(f, app, f.area());
    }
    if app.showing_priorities {
        super::priorities::draw_priorities(f, app, f.area());
    }
    if app.showing_sprints {
        super::sprints::draw_sprints(f, app, f.area());
    }
    if app.showing_mention_picker {
        super::mentions::draw_mentions(f, app, f.area());
    }
    if app.show_site_errors {
        super::errors::draw_site_errors(f, app, f.area());
    }
    if app.showing_add_link {
        let options = app.add_link_options();
        super::issue_links::draw_add_link_picker(
            f,
            &options,
            app.add_link_selected,
            &app.theme,
            f.area(),
        );
    }
    if app.showing_transition_field {
        if app.transition_multi_mode {
            super::field_picker::draw_multi_field_picker(
                f,
                &app.transition_field_heading,
                &app.transition_field_options,
                &app.transition_multi_picked,
                app.transition_field_selected,
                &app.theme,
                f.area(),
            );
        } else {
            let modal_title = if app
                .create_session
                .as_ref()
                .is_some_and(|s| s.showing_required_field)
            {
                "Create issue"
            } else {
                "Required field"
            };
            super::field_picker::draw_field_picker(
                f,
                modal_title,
                &app.transition_field_heading,
                app.transition_field_current
                    .as_ref()
                    .map(|f| f.modal_hint())
                    .unwrap_or(""),
                &app.transition_field_options,
                app.transition_field_selected,
                app.transition_field_text_mode,
                app.transition_field_current.as_ref().is_some_and(|f| {
                    f.kind == crate::api::transition_fields::TransitionFieldKind::User
                }),
                &app.theme,
                f.area(),
            );
        }
    }

    render_footer(f, app, footer_area);
}

fn render_header(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let count = app.filtered_count();
    let sites = app.sites_str();
    let status = app.refresh_status_label();
    let right_text = format!(" {count} tickets | {status}");

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
        let is_active = !app.is_custom_view_active() && tab == app.active_view;
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
    for (key, index) in app.config.custom_view_keys() {
        let is_active = app.custom_view_index == Some(index);
        let name = app.config.views.custom[index].name.as_str();
        let style = if is_active {
            Style::default()
                .fg(app.theme.accent)
                .add_modifier(ratatui::style::Modifier::BOLD)
        } else {
            Style::default().fg(app.theme.border)
        };
        let label = format!("{key}:{name}");
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
        let hint = if app.showing_mention_picker {
            " @mention"
        } else {
            " (@ to tag)"
        };
        (
            format!(" Comment{hint}: {}_", app.input_buffer),
            app.theme.accent,
        )
    } else if app.input_mode == crate::app::InputMode::Worklog {
        (
            format!(" Worklog (e.g. 30m): {}_", app.input_buffer),
            app.theme.accent,
        )
    } else if app.input_mode == crate::app::InputMode::EditSummary {
        (format!(" Summary: {}_", app.input_buffer), app.theme.accent)
    } else if app.input_mode == crate::app::InputMode::EditLabels {
        (
            format!(" Labels (comma-separated): {}_", app.input_buffer),
            app.theme.accent,
        )
    } else if app.input_mode == crate::app::InputMode::EditDueDate {
        (
            format!(
                " Due date (YYYY-MM-DD, empty clears): {}_",
                app.input_buffer
            ),
            app.theme.accent,
        )
    } else if app.input_mode == crate::app::InputMode::AddIssueLinkTarget {
        (
            format!(" Link target issue key: {}_", app.input_buffer),
            app.theme.accent,
        )
    } else if app.input_mode == crate::app::InputMode::CreateSubtaskSummary {
        (
            format!(" Subtask summary: {}_", app.input_buffer),
            app.theme.accent,
        )
    } else if app.input_mode == crate::app::InputMode::OpenTicket {
        (
            format!(
                " Open ticket (key or URL, paste from clipboard): {}_",
                app.input_buffer
            ),
            app.theme.accent,
        )
    } else if app.input_mode == crate::app::InputMode::CreateField {
        let label = if app
            .create_session
            .as_ref()
            .is_some_and(|s| s.showing_required_field)
        {
            app.transition_field_heading.as_str()
        } else {
            "Summary"
        };
        (format!(" {label}: {}_", app.input_buffer), app.theme.accent)
    } else if app.input_mode == crate::app::InputMode::CreateDescription {
        (
            format!(
                " Description (markdown, Enter empty OK): {}_",
                app.input_buffer
            ),
            app.theme.accent,
        )
    } else if app.input_mode == crate::app::InputMode::TemplateExportName {
        (
            format!(
                " Template name (saved to config, Enter): {}_",
                app.input_buffer
            ),
            app.theme.accent,
        )
    } else if app.input_mode == crate::app::InputMode::TemplateEditSummary {
        (
            format!(" Template summary (Enter): {}_", app.input_buffer),
            app.theme.accent,
        )
    } else if app.input_mode == crate::app::InputMode::TemplateEditProject {
        (
            format!(" Template project (Enter): {}_", app.input_buffer),
            app.theme.accent,
        )
    } else if app.input_mode == crate::app::InputMode::TemplateEditIssueType {
        (
            format!(" Template issue type (Enter): {}_", app.input_buffer),
            app.theme.accent,
        )
    } else if app.input_mode == crate::app::InputMode::ClosedSearchQuery {
        let scope = if app.closed_search_ever_assigned {
            "ever assigned"
        } else {
            "closed assignee"
        };
        (
            format!(" Closed search ({scope}, Enter): {}_", app.input_buffer),
            app.theme.accent,
        )
    } else if app.active_view == ViewMode::ClosedSearch
        && app.input_mode == crate::app::InputMode::None
        && app.closed_search_query.is_empty()
    {
        (
            " Closed: / search done tickets · h toggle ever-assigned · 6 tab".to_string(),
            app.theme.border,
        )
    } else if app.active_view == ViewMode::ClosedSearch
        && app.input_mode == crate::app::InputMode::None
    {
        let scope = if app.closed_search_ever_assigned {
            "ever assigned"
        } else {
            "assignee"
        };
        (
            format!(
                " Closed [{scope}]: \"{}\" — / refine · f filter results · h scope · r refresh",
                app.closed_search_query
            ),
            app.theme.border,
        )
    } else if app.input_mode == crate::app::InputMode::TransitionField {
        let (label, hint) = app
            .transition_field_current
            .as_ref()
            .map(|f| (f.name.as_str(), f.input_hint()))
            .unwrap_or(("Field", "text"));
        (
            format!(" {label} ({hint}): {}_", app.input_buffer),
            app.theme.accent,
        )
    } else if app.input_mode == crate::app::InputMode::EditDescription {
        let hint = if app.showing_mention_picker {
            " @mention"
        } else {
            " (markdown, @)"
        };
        (
            format!(" Description{hint}: {}_", app.input_buffer),
            app.theme.accent,
        )
    } else if app.loading && !app.showing_transition_field {
        let msg = app.loading_message.as_deref().unwrap_or(" Loading...");
        (format!(" {msg}"), app.theme.loading_fg)
    } else if crate::api::retry::rate_limit_active() {
        let secs = crate::api::retry::rate_limit_secs_remaining().unwrap_or(1);
        (
            format!(" Jira rate limit — wait ~{secs}s, then r to retry"),
            app.theme.loading_fg,
        )
    } else if let Some(ref msg) = app.status.action_notice {
        (format!(" {msg}"), app.theme.accent)
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
        let mut left = " ? help  / filter  ! errors  j/k  s/S sort  y copy  o/O open  t/T status  [ ] scroll  ←/→ view  1-6 tabs  R reload  q quit".to_string();
        if app.detail_open {
            left.push_str("  S/P/L/M/D fields  h/l tabs");
            if app.detail_tab == crate::app::DetailTab::Links {
                left.push_str("  Enter jump  o open  Shift+I remove  Shift+N subtask");
            }
        }
        let total = app.filtered_count();
        let row = if total == 0 { 0 } else { app.selected + 1 };
        let cache = app.cache_age_suffix();
        let right = format!(
            " {} | {row}/{total} | Sort: {}{cache}",
            app.active_view.label(),
            app.sort_mode.display(app.sort_order),
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
