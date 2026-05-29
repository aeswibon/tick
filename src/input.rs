use crossterm::event::KeyCode;

use crate::app::{App, InputMode, ViewMode};

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
                app.selected = 0;
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

    if app.showing_transitions {
        handle_transition_key(app, code).await;
        return false;
    }

    handle_normal_key(app, code).await
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
        InputMode::None => return,
    };

    match result {
        Ok(()) => app.refresh_all().await,
        Err(e) => app.status.set_action_error(e),
    }
}

async fn handle_transition_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Char(n) if n >= '1' && n <= '9' => {
            let idx = (n as u8 - b'1') as usize;
            if idx < app.transition_options.len() {
                let (trans_id, _) = app.transition_options[idx].clone();
                let Some(sel) = app.selected_ticket() else {
                    return;
                };
                let Some(base_url) = app.site_base_url(&sel.site) else {
                    return;
                };
                match app.jira.transition_issue(&base_url, &sel.key, &trans_id).await {
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
        }
        KeyCode::Esc => app.showing_transitions = false,
        _ => {}
    }
}

async fn handle_normal_key(app: &mut App, code: KeyCode) -> bool {
    match code {
        KeyCode::Char('q') => return true,
        KeyCode::Char('r') => {
            app.refresh().await;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if app.selected > 0 {
                app.selected -= 1;
            } else {
                app.prev_page();
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let visible_count = app.visible_indices().len();
            if visible_count > 0 && app.selected + 1 < visible_count {
                app.selected += 1;
            } else {
                app.next_page();
            }
        }
        KeyCode::Char('[') => app.prev_page(),
        KeyCode::Char(']') => app.next_page(),
        KeyCode::Char('/') => {
            app.filtering = true;
            app.filter.clear();
            app.detail_open = false;
            app.current_page = 0;
            app.selected = 0;
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
        }
        KeyCode::Char('?') => {
            app.show_help = !app.show_help;
            app.detail_open = false;
        }
        KeyCode::Char('s') => {
            app.sort_mode = app.sort_mode.next();
            app.current_page = 0;
            app.selected = 0;
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
        KeyCode::Char('g') => {
            app.current_page = 0;
            app.selected = 0;
        }
        KeyCode::Char('G') => app.go_to_last(),
        KeyCode::Tab => app.switch_to(app.active_view.next()).await,
        KeyCode::BackTab => app.switch_to(app.active_view.prev()).await,
        _ => {}
    }
    false
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
            app.showing_transitions = true;
        }
        Ok(_) => app.status.set_action_error("No transitions available"),
        Err(e) => app.status.set_action_error(e),
    }
}
