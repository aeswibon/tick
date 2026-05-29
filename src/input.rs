use crossterm::event::KeyCode;

use crate::app::{App, InputMode};
use crate::view_mode::ViewMode;

/// Returns `true` when the app should quit.
pub async fn handle_key(app: &mut App, code: KeyCode) -> bool {
    if app.showing_mention_picker {
        handle_mention_picker_key(app, code).await;
        return false;
    }

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
            KeyCode::Char(c) => {
                app.input_buffer.push(c);
                if mentions_enabled(app.input_mode) {
                    refresh_mention_picker(app).await;
                }
            }
            KeyCode::Backspace => {
                app.input_buffer.pop();
                if mentions_enabled(app.input_mode) {
                    refresh_mention_picker(app).await;
                }
            }
            KeyCode::Esc => {
                clear_mention_picker(app);
                app.input_mode = InputMode::None;
                app.input_buffer.clear();
                app.input_mentions.clear();
            }
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

    if app.showing_sprints {
        handle_sprint_key(app, code).await;
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

fn clear_mention_picker(app: &mut App) {
    app.showing_mention_picker = false;
    app.mention_options.clear();
    app.mention_anchor = None;
    app.mention_selected = 0;
}

pub(crate) fn active_mention_query(buffer: &str) -> Option<(usize, &str)> {
    let anchor = buffer.char_indices().rfind(|(_, c)| *c == '@')?.0;
    let query = &buffer[anchor + 1..];
    if query.chars().any(char::is_whitespace) {
        return None;
    }
    Some((anchor, query))
}

async fn refresh_mention_picker(app: &mut App) {
    let Some((anchor, query)) = active_mention_query(&app.input_buffer) else {
        clear_mention_picker(app);
        return;
    };
    app.mention_anchor = Some(anchor);
    let Some(sel) = app.selected_ticket() else {
        clear_mention_picker(app);
        return;
    };
    let Some(base_url) = app.site_base_url(&sel.site) else {
        clear_mention_picker(app);
        return;
    };
    match app
        .jira
        .search_assignable_users(&base_url, &sel.key, query)
        .await
    {
        Ok(users) => {
            app.mention_options = users;
            app.mention_selected = app
                .mention_selected
                .min(app.mention_options.len().saturating_sub(1));
            app.showing_mention_picker = true;
        }
        Err(_) => {
            app.mention_options.clear();
            app.showing_mention_picker = true;
        }
    }
}

fn confirm_mention_pick(app: &mut App) {
    let Some(anchor) = app.mention_anchor else {
        return;
    };
    let Some((account_id, display_name)) = app.mention_options.get(app.mention_selected).cloned()
    else {
        return;
    };
    let label = format!("@{display_name}");
    let prefix = &app.input_buffer[..anchor];
    app.input_buffer = format!("{prefix}{label} ");
    app.input_mentions.push((label, account_id));
    clear_mention_picker(app);
}

fn mentions_enabled(mode: InputMode) -> bool {
    matches!(mode, InputMode::Comment | InputMode::EditDescription)
}

async fn handle_mention_picker_key(app: &mut App, code: KeyCode) {
    let count = app.mention_options.len();
    match code {
        KeyCode::Esc => clear_mention_picker(app),
        KeyCode::Up | KeyCode::Char('k') if count > 0 => {
            app.mention_selected = app.mention_selected.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') if count > 0 && app.mention_selected + 1 < count => {
            app.mention_selected += 1;
        }
        KeyCode::Enter if count > 0 => confirm_mention_pick(app),
        KeyCode::Char(c) => {
            app.input_buffer.push(c);
            refresh_mention_picker(app).await;
        }
        KeyCode::Backspace => {
            app.input_buffer.pop();
            refresh_mention_picker(app).await;
        }
        _ => {}
    }
}

async fn submit_input(app: &mut App) {
    let buffer = app.input_buffer.clone();
    let mode = app.input_mode;
    let mentions = app.input_mentions.clone();
    app.input_mode = InputMode::None;
    app.input_buffer.clear();
    clear_mention_picker(app);
    app.input_mentions.clear();

    let Some(sel) = app.selected_ticket() else {
        return;
    };
    let Some(base_url) = app.site_base_url(&sel.site) else {
        app.status.set_action_error("Unknown site for ticket");
        return;
    };

    let result = match mode {
        InputMode::Comment => {
            app.jira
                .add_comment(&base_url, &sel.key, &buffer, &mentions)
                .await
        }
        InputMode::Worklog => app.jira.add_worklog(&base_url, &sel.key, &buffer).await,
        InputMode::EditSummary => app.jira.update_summary(&base_url, &sel.key, &buffer).await,
        InputMode::EditLabels => {
            let labels = crate::app::parse_labels_input(&buffer);
            app.jira.update_labels(&base_url, &sel.key, &labels).await
        }
        InputMode::EditDescription => {
            app.jira
                .update_description(&base_url, &sel.key, &buffer, &mentions)
                .await
        }
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

async fn handle_sprint_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Up | KeyCode::Char('k') => {
            app.sprint_selected = app.sprint_selected.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.sprint_selected + 1 < app.sprint_options.len() {
                app.sprint_selected += 1;
            }
        }
        KeyCode::Enter => {
            apply_sprint_move(app, app.sprint_selected).await;
        }
        KeyCode::Char(n) if ('1'..='9').contains(&n) => {
            let idx = (n as u8 - b'1') as usize;
            apply_sprint_move(app, idx).await;
        }
        KeyCode::Esc => app.showing_sprints = false,
        _ => {}
    }
}

async fn apply_sprint_move(app: &mut App, idx: usize) {
    if idx >= app.sprint_options.len() {
        return;
    }
    let (target_id, _) = app.sprint_options[idx].clone();
    let Some(sel) = app.selected_ticket() else {
        return;
    };
    let Some(base_url) = app.site_base_url(&sel.site) else {
        return;
    };
    match app
        .jira
        .move_issue_to_sprint_target(&base_url, &sel.key, &target_id)
        .await
    {
        Ok(()) => {
            app.showing_sprints = false;
            app.refresh_all().await;
        }
        Err(e) => {
            app.status.set_action_error(e);
            app.showing_sprints = false;
        }
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
        KeyCode::Char('5') => app.switch_to(ViewMode::Sprint).await,
        KeyCode::Char('t') => start_transition_picker(app).await,
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
                app.input_mentions.clear();
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

async fn start_sprint_picker(app: &mut App) {
    let Some(ticket) = app.selected_ticket_entry() else {
        return;
    };
    let Some(base_url) = app.site_base_url(&ticket.site) else {
        return;
    };
    let project = ticket.project_key_for_api().to_string();
    let board = app
        .config
        .sites
        .iter()
        .find(|s| s.name == ticket.site)
        .map(|s| s.board_config());
    match app
        .jira
        .list_sprint_targets(&base_url, &project, board.as_ref())
        .await
    {
        Ok(options) if options.len() > 1 => {
            let current = ticket.sprint_name.as_deref().unwrap_or("");
            app.sprint_options = options;
            app.sprint_selected = app
                .sprint_options
                .iter()
                .position(|(_, name)| name.contains(current) && !current.is_empty())
                .unwrap_or(0);
            app.showing_sprints = true;
        }
        Ok(_) => app
            .status
            .set_action_error("No sprints available for this project"),
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

#[cfg(test)]
mod mention_tests {
    use super::active_mention_query;

    #[test]
    fn detects_query_after_at() {
        let (pos, q) = active_mention_query("hello @ali").unwrap();
        assert_eq!(pos, 6);
        assert_eq!(q, "ali");
    }

    #[test]
    fn rejects_completed_mention_with_space() {
        assert!(active_mention_query("hey @Alice done").is_none());
    }

    #[test]
    fn uses_last_at_sign() {
        let (pos, q) = active_mention_query("@a @bob").unwrap();
        assert_eq!(pos, 3);
        assert_eq!(q, "bob");
    }

    #[test]
    fn empty_query_after_at_is_valid() {
        let (pos, q) = active_mention_query("cc @").unwrap();
        assert_eq!(pos, 3);
        assert_eq!(q, "");
    }
}
