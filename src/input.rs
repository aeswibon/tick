use crossterm::event::KeyCode;

use crate::app::{App, InputMode};
use crate::view_mode::ViewMode;

/// Returns `true` when the app should quit.
pub async fn handle_key(app: &mut App, code: KeyCode) -> bool {
    if app.filtering {
        match code {
            KeyCode::Char(c) => app.filter.push(c),
            KeyCode::Backspace => {
                app.filter.pop();
            }
            KeyCode::Esc | KeyCode::Enter => {
                app.filtering = false;
                app.go_to_first();
                app.invalidate_filter_cache();
            }
            _ => {}
        }
        return false;
    }

    if app.input_mode != InputMode::None {
        match code {
            KeyCode::Char(c) => app.input_buffer.push(c),
            KeyCode::Backspace => {
                app.input_buffer.pop();
            }
            KeyCode::Esc => app.input_mode = InputMode::None,
            KeyCode::Enter => {
                submit_input(app).await;
            }
            _ => {}
        }
        return false;
    }

    if app.show_site_errors {
        handle_site_errors_key(app, code);
        return false;
    }

    if app.showing_transitions {
        handle_transition_key(app, code).await;
        return false;
    }

    if app.showing_priorities {
        handle_priority_key(app, code).await;
        return false;
    }

    handle_normal_key(app, code).await
}

fn handle_site_errors_key(app: &mut App, code: KeyCode) {
    let count = app.status.site_warnings.len();
    match code {
        KeyCode::Esc | KeyCode::Char('!') => {
            app.show_site_errors = false;
            app.site_error_scroll = 0;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.site_error_scroll = app.site_error_scroll.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') if app.site_error_scroll + 1 < count => {
            app.site_error_scroll += 1;
        }
        _ => {}
    }
}

async fn submit_input(app: &mut App) {
    let buffer = app.input_buffer.clone();
    let mode = app.input_mode;
    app.input_mode = InputMode::None;
    app.input_buffer.clear();

    let Some(sel) = app.selected_ticket() else {
        return;
    };
    let Some(base_url) = app.site_base_url(&sel.site) else {
        app.status.set_action_error("Unknown site for ticket");
        return;
    };

    let result = match mode {
        InputMode::Comment => app.jira.add_comment(&base_url, &sel.key, &buffer).await,
        InputMode::Worklog => app.jira.add_worklog(&base_url, &sel.key, &buffer).await,
        InputMode::EditSummary => app.jira.update_summary(&base_url, &sel.key, &buffer).await,
        InputMode::None => return,
    };

    match result {
        Ok(()) => app.refresh_all().await,
        Err(e) => app.status.set_action_error(e),
    }
}

async fn handle_transition_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Up | KeyCode::Char('k') => {
            app.transition_selected = app.transition_selected.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.transition_selected + 1 < app.transition_options.len() {
                app.transition_selected += 1;
            }
        }
        KeyCode::Enter => {
            apply_transition(app, app.transition_selected).await;
        }
        KeyCode::Char(n) if ('1'..='9').contains(&n) => {
            let idx = (n as u8 - b'1') as usize;
            apply_transition(app, idx).await;
        }
        KeyCode::Esc => app.showing_transitions = false,
        _ => {}
    }
}

async fn handle_priority_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Up | KeyCode::Char('k') => {
            app.priority_selected = app.priority_selected.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.priority_selected + 1 < app.priority_options.len() {
                app.priority_selected += 1;
            }
        }
        KeyCode::Enter => {
            apply_priority(app, app.priority_selected).await;
        }
        KeyCode::Char(n) if ('1'..='9').contains(&n) => {
            let idx = (n as u8 - b'1') as usize;
            apply_priority(app, idx).await;
        }
        KeyCode::Esc => app.showing_priorities = false,
        _ => {}
    }
}

async fn apply_priority(app: &mut App, idx: usize) {
    if idx >= app.priority_options.len() {
        return;
    }
    let (_, name) = app.priority_options[idx].clone();
    let Some(sel) = app.selected_ticket() else {
        return;
    };
    let Some(base_url) = app.site_base_url(&sel.site) else {
        return;
    };
    match app.jira.update_priority(&base_url, &sel.key, &name).await {
        Ok(()) => {
            app.showing_priorities = false;
            app.refresh_all().await;
        }
        Err(e) => {
            app.status.set_action_error(e);
            app.showing_priorities = false;
        }
    }
}

async fn apply_transition(app: &mut App, idx: usize) {
    if idx >= app.transition_options.len() {
        return;
    }
    let (trans_id, _) = app.transition_options[idx].clone();
    let Some(sel) = app.selected_ticket() else {
        return;
    };
    let Some(base_url) = app.site_base_url(&sel.site) else {
        return;
    };
    match app
        .jira
        .transition_issue(&base_url, &sel.key, &trans_id)
        .await
    {
        Ok(()) => {
            app.showing_transitions = false;
            app.refresh_all().await;
        }
        Err(e) => {
            app.status.set_action_error(e);
            app.showing_transitions = false;
        }
    }
}

async fn handle_normal_key(app: &mut App, code: KeyCode) -> bool {
    match code {
        KeyCode::Char('q') => return true,
        KeyCode::Char('r') => {
            app.refresh().await;
        }
        KeyCode::Up | KeyCode::Char('k') => app.move_selection_up(),
        KeyCode::Down | KeyCode::Char('j') => app.move_selection_down(),
        KeyCode::Char('[') => app.scroll_page_up(),
        KeyCode::Char(']') => app.scroll_page_down(),
        KeyCode::Char('/') => {
            app.filtering = true;
            app.filter.clear();
            app.detail_open = false;
            app.go_to_first();
            app.invalidate_filter_cache();
        }
        KeyCode::Enter => {
            if app.show_help {
                app.show_help = false;
            } else if !app.detail_open {
                app.detail_open = true;
            }
        }
        KeyCode::Esc => {
            app.show_help = false;
            app.detail_open = false;
            app.showing_transitions = false;
            app.showing_priorities = false;
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
        KeyCode::Char('h') if app.detail_open => {
            app.detail_tab = app.detail_tab.prev();
        }
        KeyCode::Char('l') if app.detail_open => {
            app.detail_tab = app.detail_tab.next();
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
        KeyCode::Char('o') if !app.detail_open => {
            if let Some(sel) = app.selected_ticket() {
                if crate::platform::open_url(&sel.link).is_err() {
                    app.status.set_action_error("Could not open browser");
                }
            }
        }
        KeyCode::Char('1') => app.switch_to(ViewMode::MyIssues).await,
        KeyCode::Char('2') => app.switch_to(ViewMode::Updated).await,
        KeyCode::Char('3') => app.switch_to(ViewMode::Mentions).await,
        KeyCode::Char('4') => app.switch_to(ViewMode::Watching).await,
        KeyCode::Char('t') => start_transition_picker(app).await,
        KeyCode::Char('c') if app.detail_open => {
            app.input_mode = InputMode::Comment;
            app.input_buffer.clear();
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
        KeyCode::Char('g') => app.go_to_first(),
        KeyCode::Char('G') => app.go_to_last(),
        KeyCode::Tab => app.switch_to(app.active_view.next()).await,
        KeyCode::BackTab => app.switch_to(app.active_view.prev()).await,
        _ => {}
    }
    false
}

async fn assign_to_me(app: &mut App) {
    let Some(sel) = app.selected_ticket() else {
        return;
    };
    let Some(base_url) = app.site_base_url(&sel.site) else {
        app.status.set_action_error("Unknown site for ticket");
        return;
    };
    match app.jira.current_user_account_id(&base_url).await {
        Ok(account_id) => match app
            .jira
            .assign_to_account(&base_url, &sel.key, &account_id)
            .await
        {
            Ok(()) => app.refresh_all().await,
            Err(e) => app.status.set_action_error(e),
        },
        Err(e) => app.status.set_action_error(e),
    }
}

async fn unassign_ticket(app: &mut App) {
    let Some(sel) = app.selected_ticket() else {
        return;
    };
    let Some(base_url) = app.site_base_url(&sel.site) else {
        app.status.set_action_error("Unknown site for ticket");
        return;
    };
    match app.jira.unassign(&base_url, &sel.key).await {
        Ok(()) => app.refresh_all().await,
        Err(e) => app.status.set_action_error(e),
    }
}

async fn start_priority_picker(app: &mut App) {
    let Some(sel) = app.selected_ticket() else {
        return;
    };
    let Some(base_url) = app.site_base_url(&sel.site) else {
        return;
    };
    match app.jira.list_priorities(&base_url).await {
        Ok(options) if !options.is_empty() => {
            let current = app
                .selected_ticket_entry()
                .map(|t| t.priority)
                .unwrap_or_default();
            app.priority_options = options;
            app.priority_selected = app
                .priority_options
                .iter()
                .position(|(_, name)| name == &current)
                .unwrap_or(0);
            app.showing_priorities = true;
        }
        Ok(_) => app.status.set_action_error("No priorities available"),
        Err(e) => app.status.set_action_error(e),
    }
}

async fn start_transition_picker(app: &mut App) {
    let Some(sel) = app.selected_ticket() else {
        return;
    };
    let Some(base_url) = app.site_base_url(&sel.site) else {
        return;
    };
    match app.jira.get_transition_options(&base_url, &sel.key).await {
        Ok(options) if !options.is_empty() => {
            app.transition_options = options;
            app.transition_selected = 0;
            app.showing_transitions = true;
        }
        Ok(_) => app.status.set_action_error("No transitions available"),
        Err(e) => app.status.set_action_error(e),
    }
}
