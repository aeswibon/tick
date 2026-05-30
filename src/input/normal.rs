use crossterm::event::KeyCode;

use crate::app::{App, InputMode};
use crate::ticket_lock::read_tickets;
use crate::view_mode::ViewMode;

use super::detail_actions::{
    assign_to_me, start_priority_picker, start_sprint_picker, start_status_picker, unassign_ticket,
};
use super::mentions::{clear_mention_picker, start_open_ticket};
use super::transitions::cancel_transition_collect;

pub(crate) async fn handle_normal_key(app: &mut App, code: KeyCode) -> bool {
    match code {
        KeyCode::Char('q') => return true,
        KeyCode::Char('r') => {
            app.refresh().await;
        }
        KeyCode::Char('R') if !app.detail_open => match app.reload_config(app.debug).await {
            Ok(()) => {}
            Err(e) => app.status.set_action_error(e),
        },
        KeyCode::Up | KeyCode::Char('k') => {
            if app.detail_open && app.detail_tab == crate::app::DetailTab::Links {
                app.links_selected = app.links_selected.saturating_sub(1);
            } else {
                app.move_selection_up();
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.detail_open && app.detail_tab == crate::app::DetailTab::Links {
                if app.links_selected + 1 < app.links_row_count() {
                    app.links_selected += 1;
                }
            } else {
                app.move_selection_down();
            }
        }
        KeyCode::Char('[') => app.scroll_page_up(),
        KeyCode::Char(']') => app.scroll_page_down(),
        KeyCode::Char('/') => {
            app.detail_open = false;
            if app.active_view == ViewMode::ClosedSearch && !app.is_custom_view_active() {
                app.input_mode = InputMode::ClosedSearchQuery;
                app.input_buffer = app.closed_search_query.clone();
            } else {
                app.filtering = true;
                app.filter.clear();
                app.go_to_first();
                app.invalidate_filter_cache();
            }
        }
        KeyCode::Char('f')
            if app.active_view == ViewMode::ClosedSearch
                && !app.is_custom_view_active()
                && !read_tickets(&app.tickets).is_empty() =>
        {
            app.filtering = true;
            app.filter.clear();
            app.go_to_first();
            app.invalidate_filter_cache();
        }
        KeyCode::Enter => {
            if app.show_help {
                app.show_help = false;
            } else if app.detail_open && app.detail_tab == crate::app::DetailTab::Links {
                crate::issue_relations_flow::jump_to_selected_link(app).await;
            } else if !app.detail_open {
                app.detail_open = true;
                app.refresh_issue_relations().await;
            }
        }
        KeyCode::Esc => {
            app.show_help = false;
            app.detail_open = false;
            crate::issue_relations_flow::cancel_add_link(app);
            app.showing_transitions = false;
            cancel_transition_collect(app);
            crate::create_flow::cancel_create(app);
            app.showing_priorities = false;
            app.showing_sprints = false;
            app.show_site_errors = false;
        }
        KeyCode::Char('!') if app.status.has_warnings() => {
            app.show_site_errors = !app.show_site_errors;
            app.site_error_scroll = 0;
        }
        KeyCode::Char('?') => {
            app.show_help = !app.show_help;
            app.detail_open = false;
        }
        KeyCode::Char('s') => {
            app.sort_mode = app.sort_mode.next();
            app.go_to_first();
            app.invalidate_filter_cache();
        }
        KeyCode::Char('S') if !app.detail_open => {
            if app.sort_mode != crate::app::SortMode::Default {
                app.sort_order = app.sort_order.toggle();
                app.go_to_first();
                app.invalidate_filter_cache();
            }
        }
        KeyCode::Char('h') if app.active_view == ViewMode::ClosedSearch && !app.detail_open => {
            app.toggle_closed_search_history();
            let mode = if app.closed_search_ever_assigned {
                "ever assigned to you"
            } else {
                "assignee when closed"
            };
            app.status
                .set_action_notice(format!("Closed search: {mode}"));
            if !app.closed_search_query.trim().is_empty() {
                app.refresh_closed_search().await;
            }
        }
        KeyCode::Char('h') if app.detail_open => {
            app.detail_tab = app.detail_tab.prev();
            if app.detail_tab == crate::app::DetailTab::Links {
                app.refresh_issue_relations().await;
            }
        }
        KeyCode::Char('l') if app.detail_open => {
            app.detail_tab = app.detail_tab.next();
            if app.detail_tab == crate::app::DetailTab::Links {
                app.refresh_issue_relations().await;
            }
        }
        KeyCode::Right => app.switch_to(app.active_view.next()).await,
        KeyCode::Left => app.switch_to(app.active_view.prev()).await,
        KeyCode::Char('e') => {
            if let Ok(path) = crate::config::Config::config_path() {
                let _ = crate::platform::open_path(&path);
            }
        }
        KeyCode::Char('y') => {
            if let Some(sel) = app.selected_ticket() {
                if !crate::platform::copy_to_clipboard(&sel.key) {
                    app.status
                        .set_action_error("Clipboard unavailable on this system");
                }
            }
        }
        KeyCode::Char('o') if app.detail_open && app.detail_tab == crate::app::DetailTab::Links => {
            crate::issue_relations_flow::open_selected_link_in_browser(app).await;
        }
        KeyCode::Char('o') if !app.detail_open => {
            if let Some(sel) = app.selected_ticket() {
                if crate::platform::open_url(&sel.link).is_err() {
                    app.status.set_action_error("Could not open browser");
                }
            }
        }
        KeyCode::Char('O') if !app.detail_open => {
            start_open_ticket(app).await;
        }
        KeyCode::Char('1') => app.switch_to(ViewMode::MyIssues).await,
        KeyCode::Char('2') => app.switch_to(ViewMode::Mentions).await,
        KeyCode::Char('3') => app.switch_to(ViewMode::Watching).await,
        KeyCode::Char('4') => app.switch_to(ViewMode::Updated).await,
        KeyCode::Char('5') => app.switch_to(ViewMode::Sprint).await,
        KeyCode::Char('6') => app.switch_to(ViewMode::ClosedSearch).await,
        KeyCode::Char('7') => try_switch_custom_key(app, 7).await,
        KeyCode::Char('8') => try_switch_custom_key(app, 8).await,
        KeyCode::Char('9') => try_switch_custom_key(app, 9).await,
        KeyCode::Char('n') if !app.detail_open && app.create_session.is_none() => {
            crate::create_flow::start_create_blank(app).await;
        }
        KeyCode::Char('N') if !app.detail_open && app.create_session.is_none() => {
            crate::create_flow::start_create_from_template(app).await;
        }
        KeyCode::Char('C') if !app.detail_open && app.create_session.is_none() => {
            crate::create_flow::start_create_duplicate(app).await;
        }
        KeyCode::Char('X') if app.create_session.is_none() && app.template_export.is_none() => {
            crate::template_export_flow::start_template_export_from_selection(app).await;
        }
        KeyCode::Char('t') | KeyCode::Char('T') => {
            if crate::create_flow::handle_create_normal_keys(app, code).await {
                // create wizard: change issue type
            } else {
                start_status_picker(app).await;
            }
        }
        KeyCode::Char('p') if !app.detail_open => {
            if !crate::create_flow::handle_create_normal_keys(app, code).await {
                // not in create wizard; ignore (priority uses P)
            }
        }
        KeyCode::Char('c') if app.detail_open => {
            app.input_mode = InputMode::Comment;
            app.input_buffer.clear();
            app.input_mentions.clear();
            clear_mention_picker(app);
        }
        KeyCode::Char('w') if app.detail_open => {
            app.input_mode = InputMode::Worklog;
            app.input_buffer.clear();
        }
        KeyCode::Char('a') if app.detail_open => {
            assign_to_me(app).await;
        }
        KeyCode::Char('u') if app.detail_open => {
            unassign_ticket(app).await;
        }
        KeyCode::Char('S') if app.detail_open => {
            if let Some(ticket) = app.selected_ticket_entry() {
                app.input_mode = InputMode::EditSummary;
                app.input_buffer = ticket.summary;
            }
        }
        KeyCode::Char('P') if app.detail_open => {
            start_priority_picker(app).await;
        }
        KeyCode::Char('L') if app.detail_open => {
            if let Some(ticket) = app.selected_ticket_entry() {
                app.input_mode = InputMode::EditLabels;
                app.input_buffer = ticket.labels.join(", ");
            }
        }
        KeyCode::Char('M') if app.detail_open => {
            start_sprint_picker(app).await;
        }
        KeyCode::Char('d') if app.detail_open => {
            if let Some(ticket) = app.selected_ticket_entry() {
                app.input_mode = InputMode::EditDueDate;
                app.input_buffer = ticket
                    .due_date
                    .map(|d| d.format("%Y-%m-%d").to_string())
                    .unwrap_or_default();
            }
        }
        KeyCode::Char('D') if app.detail_open => {
            if let Some(ticket) = app.selected_ticket_entry() {
                let text = ticket
                    .description_adf
                    .as_ref()
                    .map(crate::api::adf_export::to_markdown)
                    .filter(|s| !s.is_empty())
                    .or(ticket.description.clone())
                    .unwrap_or_default();
                app.input_mode = InputMode::EditDescription;
                app.input_buffer = text;
                app.input_mentions = ticket
                    .description_adf
                    .as_ref()
                    .map(crate::api::types::collect_mentions)
                    .unwrap_or_default();
                clear_mention_picker(app);
            }
        }
        KeyCode::Char('g') => app.go_to_first(),
        KeyCode::Char('G') => app.go_to_last(),
        KeyCode::Tab => app.switch_to(app.active_view.next()).await,
        KeyCode::BackTab => app.switch_to(app.active_view.prev()).await,
        _ => {}
    }
    false
}
pub(crate) async fn try_switch_custom_key(app: &mut App, key: u8) {
    if let Some((_, index)) = app
        .config
        .custom_view_keys()
        .into_iter()
        .find(|(k, _)| *k == key)
    {
        app.switch_to_custom(index).await;
    }
}
