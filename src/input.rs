use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use std::collections::HashMap;

use crate::api::assignable_users;
use crate::api::transition_fields::{self, TransitionField, TransitionFieldKind};
use crate::api::{self, types::WorkflowTransition};
use crate::app::{App, InputMode, TransitionCollect};
use crate::ticket_lock::read_tickets;
use crate::view_mode::ViewMode;

/// Shown in picker footers (⌘R on macOS; Ctrl+R elsewhere). Both work on macOS when the terminal reports modifiers.
#[cfg(target_os = "macos")]
pub const LOAD_MORE_USERS_KEYS_HINT: &str = "⌘R";
#[cfg(not(target_os = "macos"))]
pub const LOAD_MORE_USERS_KEYS_HINT: &str = "Ctrl+R";

#[cfg(target_os = "macos")]
pub const LOAD_MORE_USERS_USER_MODAL_HINT: &str =
    "Type in footer to filter; ⌘R fetch more users into cache; Enter to select";
#[cfg(not(target_os = "macos"))]
pub const LOAD_MORE_USERS_USER_MODAL_HINT: &str =
    "Type in footer to filter; Ctrl+R fetch more users into cache; Enter to select";

#[cfg(target_os = "macos")]
pub const LOAD_MORE_USERS_PICKER_FOOTER: &str = "  j/k move  Enter pick  ⌘R add users  Esc cancel";
#[cfg(not(target_os = "macos"))]
pub const LOAD_MORE_USERS_PICKER_FOOTER: &str =
    "  j/k move  Enter pick  Ctrl+R add users  Esc cancel";

#[cfg(target_os = "macos")]
pub const LOAD_MORE_USERS_FIELD_PICKER_FOOTER: &str =
    "  Type in footer to filter  j/k move  Enter pick  ⌘R add users  Esc cancel";
#[cfg(not(target_os = "macos"))]
pub const LOAD_MORE_USERS_FIELD_PICKER_FOOTER: &str =
    "  Type in footer to filter  j/k move  Enter pick  Ctrl+R add users  Esc cancel";

/// Load more assignable users from Jira (merge into cache). Plain `r`/`R` are for filtering.
pub fn load_more_users_key(key: &KeyEvent) -> bool {
    if !matches!(key.code, KeyCode::Char('r') | KeyCode::Char('R')) {
        return false;
    }
    let mods = key.modifiers;
    mods.contains(KeyModifiers::CONTROL)
        || mods.contains(KeyModifiers::SUPER)
        || mods.contains(KeyModifiers::META)
}

/// Returns `true` when the app should quit.
pub async fn handle_key(app: &mut App, key: KeyEvent) -> bool {
    let code = key.code;
    if app.showing_mention_picker {
        handle_mention_picker_key(app, &key).await;
        return false;
    }

    let create_required = app
        .create_session
        .as_ref()
        .is_some_and(|s| s.showing_required_field);
    if app.showing_add_link {
        crate::issue_relations_flow::handle_add_link_key(app, code).await;
        return false;
    }

    if app.showing_transition_field && app.transition_multi_mode {
        handle_transition_multi_field_key(app, code).await;
        return false;
    }

    if app.showing_transition_field && app.transition_field_user_search {
        if create_required {
            if crate::create_flow::handle_create_field_key(app, &key).await {
                return false;
            }
        } else if handle_transition_user_field_key(app, &key).await {
            return false;
        }
    } else if app.showing_transition_field && !app.transition_field_text_mode {
        if create_required {
            if crate::create_flow::handle_create_field_key(app, &key).await {
                return false;
            }
        } else {
            handle_transition_field_key(app, code).await;
        }
        return false;
    }

    if app.showing_create_picker {
        crate::create_flow::handle_create_picker_key(app, code).await;
        return false;
    }

    if app.template_export.is_some() && app.input_mode != InputMode::TemplateExportName {
        crate::template_export_flow::handle_template_export_key(app, code).await;
        return false;
    }

    if app.template_manage.is_some()
        && !matches!(
            app.input_mode,
            InputMode::TemplateEditSummary
                | InputMode::TemplateEditProject
                | InputMode::TemplateEditIssueType
        )
    {
        crate::template_manage_flow::handle_template_manage_key(app, code).await;
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
                } else if app.input_mode == InputMode::TransitionField
                    && app.transition_field_user_search
                {
                    refresh_transition_user_search(app, false).await;
                } else if app.input_mode == InputMode::CreateField
                    && app
                        .create_session
                        .as_ref()
                        .is_some_and(|s| s.showing_required_field)
                    && app.transition_field_user_search
                {
                    crate::create_flow::refresh_create_user_search(app, false).await;
                }
            }
            KeyCode::Backspace => {
                app.input_buffer.pop();
                if mentions_enabled(app.input_mode) {
                    refresh_mention_picker(app).await;
                } else if app.input_mode == InputMode::TransitionField
                    && app.transition_field_user_search
                {
                    refresh_transition_user_search(app, false).await;
                } else if app.input_mode == InputMode::CreateField
                    && app
                        .create_session
                        .as_ref()
                        .is_some_and(|s| s.showing_required_field)
                    && app.transition_field_user_search
                {
                    crate::create_flow::refresh_create_user_search(app, false).await;
                }
            }
            KeyCode::Esc => {
                clear_mention_picker(app);
                if app.input_mode == InputMode::TransitionField {
                    cancel_transition_collect(app);
                } else if matches!(
                    app.input_mode,
                    InputMode::CreateField | InputMode::CreateDescription
                ) {
                    crate::create_flow::cancel_create(app);
                } else if app.input_mode == InputMode::TemplateExportName {
                    crate::template_export_flow::cancel_template_export(app);
                } else if matches!(
                    app.input_mode,
                    InputMode::EditDueDate
                        | InputMode::ClosedSearchQuery
                        | InputMode::AddIssueLinkTarget
                        | InputMode::CreateSubtaskSummary
                        | InputMode::TemplateEditSummary
                        | InputMode::TemplateEditProject
                        | InputMode::TemplateEditIssueType
                ) {
                    if matches!(
                        app.input_mode,
                        InputMode::TemplateEditSummary
                            | InputMode::TemplateEditProject
                            | InputMode::TemplateEditIssueType
                    ) {
                        if let Some(session) = app.template_manage.as_mut() {
                            session.step = crate::template_manage_flow::TemplateManageStep::Actions;
                        }
                    }
                    app.input_mode = InputMode::None;
                    app.input_buffer.clear();
                } else {
                    app.input_mode = InputMode::None;
                    app.input_buffer.clear();
                    app.input_mentions.clear();
                }
            }
            KeyCode::Enter => {
                if app.input_mode == InputMode::OpenTicket {
                    submit_open_ticket(app).await;
                } else if matches!(
                    app.input_mode,
                    InputMode::CreateField | InputMode::CreateDescription
                ) {
                    crate::create_flow::submit_create_input(app).await;
                } else if app.input_mode == InputMode::TemplateExportName {
                    crate::template_export_flow::submit_template_export_name(app).await;
                } else if app.input_mode == InputMode::AddIssueLinkTarget {
                    crate::issue_relations_flow::submit_add_link_target(app).await;
                } else if app.input_mode == InputMode::CreateSubtaskSummary {
                    crate::issue_relations_flow::submit_create_subtask(app).await;
                } else if app.input_mode == InputMode::ClosedSearchQuery {
                    app.closed_search_query = app.input_buffer.trim().to_string();
                    app.input_mode = InputMode::None;
                    app.input_buffer.clear();
                    app.save_closed_prefs();
                    app.refresh_closed_search().await;
                } else if matches!(
                    app.input_mode,
                    InputMode::TemplateEditSummary
                        | InputMode::TemplateEditProject
                        | InputMode::TemplateEditIssueType
                ) {
                    crate::template_manage_flow::submit_template_edit(app).await;
                } else {
                    submit_input(app).await;
                }
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

    if code == KeyCode::Char('W') {
        if key.modifiers.contains(KeyModifiers::SHIFT) {
            unwatch_ticket(app).await;
        } else {
            watch_ticket(app).await;
        }
        return false;
    }

    if matches!(code, KeyCode::Char('I') | KeyCode::Char('i')) {
        if key.modifiers.contains(KeyModifiers::SHIFT) {
            if app.detail_open && app.detail_tab == crate::app::DetailTab::Links {
                crate::issue_relations_flow::remove_selected_link(app).await;
            }
        } else if app.detail_open {
            crate::issue_relations_flow::start_add_link(app);
        }
        return false;
    }

    if matches!(code, KeyCode::Char('N') | KeyCode::Char('n'))
        && key.modifiers.contains(KeyModifiers::SHIFT)
        && app.detail_open
        && app.detail_tab == crate::app::DetailTab::Links
    {
        crate::issue_relations_flow::start_create_subtask(app);
        return false;
    }

    if matches!(code, KeyCode::Char('v') | KeyCode::Char('V')) && !app.detail_open {
        if key.modifiers.contains(KeyModifiers::SHIFT) {
            app.cycle_custom_view(false).await;
        } else {
            app.cycle_custom_view(true).await;
        }
        return false;
    }

    if matches!(code, KeyCode::Char('E'))
        && key.modifiers.contains(KeyModifiers::SHIFT)
        && !app.detail_open
    {
        crate::template_manage_flow::start_template_manage(app);
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

async fn refresh_mention_catalog(app: &mut App, force: bool) -> Result<(), String> {
    let Some(sel) = app.selected_ticket() else {
        return Ok(());
    };
    let Some(base_url) = app.site_base_url(&sel.site) else {
        return Ok(());
    };
    let api_query = active_mention_query(&app.input_buffer)
        .map(|(_, q)| q.to_string())
        .unwrap_or_default();
    if force {
        app.loading = true;
        app.loading_message = Some("Loading more users…".into());
    }
    let result = app
        .jira
        .ensure_assignable_users(&base_url, &sel.key, &api_query, force)
        .await
        .map(|_| ());
    if force {
        app.loading = false;
        app.loading_message = None;
    }
    result
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
        .filter_assignable_users(&base_url, &sel.key, query)
        .await
    {
        Ok(users) => {
            app.mention_options = users;
            app.mention_selected = app
                .mention_selected
                .min(app.mention_options.len().saturating_sub(1));
            app.showing_mention_picker = true;
        }
        Err(e) => {
            app.status.set_action_error(e);
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

async fn handle_mention_picker_key(app: &mut App, key: &KeyEvent) {
    let code = key.code;
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
        _ if load_more_users_key(key) => {
            if refresh_mention_catalog(app, true).await.is_ok() {
                refresh_mention_picker(app).await;
            }
        }
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

async fn start_open_ticket(app: &mut App) {
    let prefilled = crate::platform::read_from_clipboard().unwrap_or_default();
    if !prefilled.is_empty() {
        app.loading = true;
        if app.config.sites.len() > 1 {
            app.loading_message = Some(format!(
                "Looking up issue ({} sites)…",
                app.config.sites.len()
            ));
        }
        let result = app.resolve_ticket_url(&prefilled).await;
        app.loading = false;
        app.loading_message = None;
        if let Ok(url) = result {
            if crate::platform::open_url(&url).is_ok() {
                return;
            }
        }
    }
    app.input_mode = InputMode::OpenTicket;
    app.input_buffer = prefilled;
}

async fn submit_open_ticket(app: &mut App) {
    let buffer = app.input_buffer.clone();
    app.input_mode = InputMode::None;
    app.input_buffer.clear();
    app.loading = true;
    if app.config.sites.len() > 1 {
        app.loading_message = Some(format!(
            "Looking up issue ({} sites)…",
            app.config.sites.len()
        ));
    }
    let result = app.resolve_ticket_url(&buffer).await;
    app.loading = false;
    app.loading_message = None;
    match result {
        Ok(url) => {
            if crate::platform::open_url(&url).is_err() {
                app.status.set_action_error("Could not open browser");
            }
        }
        Err(e) => app.status.set_action_error(e),
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
        InputMode::EditDueDate => match parse_due_date_input(&buffer) {
            Ok(due) => app.jira.update_due_date(&base_url, &sel.key, due).await,
            Err(e) => {
                app.input_mode = InputMode::EditDueDate;
                app.input_buffer = buffer;
                app.status.set_action_error(e);
                return;
            }
        },
        InputMode::TransitionField => {
            let Some(field) = app.transition_field_current.clone() else {
                return;
            };
            if field.kind == TransitionFieldKind::User && !app.transition_field_options.is_empty() {
                let idx = app.transition_field_selected;
                apply_transition_field_pick(app, idx).await;
                return;
            }
            let Some(field) = app.transition_field_current.take() else {
                return;
            };
            match field.value_from_text(&buffer) {
                Ok(value) => {
                    advance_transition_field(app, &field, value);
                    prompt_next_transition_field(app).await;
                }
                Err(e) => app.status.set_action_error(e),
            }
            return;
        }
        InputMode::CreateField
        | InputMode::CreateDescription
        | InputMode::TemplateExportName
        | InputMode::ClosedSearchQuery
        | InputMode::AddIssueLinkTarget
        | InputMode::CreateSubtaskSummary
        | InputMode::TemplateEditSummary
        | InputMode::TemplateEditProject
        | InputMode::TemplateEditIssueType => {
            return;
        }
        InputMode::OpenTicket | InputMode::None => return,
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

async fn refresh_transition_user_search(app: &mut App, force_refresh: bool) {
    if !app.transition_field_user_search {
        return;
    }
    let query = app.input_buffer.trim();

    let Some(sel) = app.selected_ticket() else {
        return;
    };
    let Some(base_url) = app.site_base_url(&sel.site) else {
        return;
    };

    if force_refresh {
        app.loading = true;
        app.loading_message = Some("Refreshing users…".into());
    }

    let api_query = if force_refresh { query } else { "" };
    let users = match app
        .jira
        .ensure_assignable_users(&base_url, &sel.key, api_query, force_refresh)
        .await
    {
        Ok(catalog) => assignable_users::filter_users(&catalog, query),
        Err(e) => {
            app.status.set_action_error(e);
            Vec::new()
        }
    };

    if force_refresh {
        app.loading = false;
        app.loading_message = None;
    }

    if !app.transition_field_user_search {
        return;
    }

    app.transition_field_options = users;
    app.transition_field_selected = app
        .transition_field_selected
        .min(app.transition_field_options.len().saturating_sub(1));
    app.showing_transition_field = true;
    // Keep footer active for filter text; list navigation uses j/k when options exist.
    app.transition_field_text_mode = false;
}

fn cancel_transition_collect(app: &mut App) {
    app.transition_collect = None;
    app.showing_transition_field = false;
    app.transition_field_text_mode = false;
    app.transition_multi_mode = false;
    app.transition_multi_picked.clear();
    app.transition_field_user_search = false;
    app.transition_field_current = None;
    app.transition_field_options.clear();
    if app.input_mode == InputMode::TransitionField {
        app.input_mode = InputMode::None;
        app.input_buffer.clear();
    }
}

fn advance_transition_field(app: &mut App, field: &TransitionField, value: serde_json::Value) {
    if let Some(collect) = &mut app.transition_collect {
        collect.values.insert(field.id.clone(), value);
        if collect.pending.first().map(|f| f.id.as_str()) == Some(field.id.as_str()) {
            collect.pending.remove(0);
        } else {
            collect.pending.retain(|f| f.id != field.id);
        }
    }
    app.showing_transition_field = false;
    app.transition_field_current = None;
    app.transition_field_user_search = false;
    if app.input_mode == InputMode::TransitionField {
        app.input_mode = InputMode::None;
        app.input_buffer.clear();
    }
}

/// Opens the next required-field prompt. Returns `false` when the queue is empty (caller should POST).
fn begin_next_field_prompt(app: &mut App) -> bool {
    let Some(collect) = app.transition_collect.as_ref() else {
        return false;
    };
    if collect.pending.is_empty() {
        return false;
    }

    let field = collect.pending[0].clone();
    let remaining = collect.pending.len();
    app.transition_field_current = Some(field.clone());
    app.transition_field_heading = if remaining > 1 {
        format!("{} ({} more)", field.name, remaining)
    } else {
        field.name.clone()
    };

    app.showing_transition_field = true;
    app.transition_field_user_search = false;

    match field.kind {
        TransitionFieldKind::User => {
            app.transition_field_text_mode = true;
            app.transition_field_user_search = true;
            app.transition_field_options.clear();
            app.input_mode = InputMode::TransitionField;
            app.input_buffer.clear();
        }
        TransitionFieldKind::Picker | TransitionFieldKind::Boolean if !field.options.is_empty() => {
            app.transition_multi_mode = false;
            app.transition_multi_picked.clear();
            app.transition_field_text_mode = false;
            app.transition_field_options = field.options;
            app.transition_field_selected = 0;
        }
        TransitionFieldKind::MultiPicker if !field.options.is_empty() => {
            app.transition_multi_mode = true;
            app.transition_multi_picked = vec![false; field.options.len()];
            app.transition_field_text_mode = false;
            app.transition_field_options = field.options;
            app.transition_field_selected = 0;
        }
        _ => {
            app.transition_multi_mode = false;
            app.transition_multi_picked.clear();
            app.transition_field_text_mode = true;
            app.transition_field_options.clear();
            app.input_mode = InputMode::TransitionField;
            app.input_buffer.clear();
        }
    }
    true
}

async fn handle_transition_multi_field_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc => cancel_transition_collect(app),
        KeyCode::Up | KeyCode::Char('k') => {
            app.transition_field_selected = app.transition_field_selected.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.transition_field_selected + 1 < app.transition_field_options.len() {
                app.transition_field_selected += 1;
            }
        }
        KeyCode::Char(' ') => {
            let i = app.transition_field_selected;
            if let Some(slot) = app.transition_multi_picked.get_mut(i) {
                *slot = !*slot;
            }
        }
        KeyCode::Enter => {
            let Some(field) = app.transition_field_current.clone() else {
                return;
            };
            let picks: Vec<_> = app
                .transition_field_options
                .iter()
                .enumerate()
                .filter(|(i, _)| {
                    app.transition_multi_picked
                        .get(*i)
                        .copied()
                        .unwrap_or(false)
                })
                .map(|(_, pair)| pair.clone())
                .collect();
            if picks.is_empty() {
                app.status
                    .set_action_error(format!("Select at least one value for {}", field.name));
                return;
            }
            let value = field.value_from_multi_choices(&picks);
            app.transition_multi_mode = false;
            app.transition_multi_picked.clear();
            let create_required = app
                .create_session
                .as_ref()
                .is_some_and(|s| s.showing_required_field);
            if create_required {
                crate::create_flow::advance_create_required(app, &field, value);
                crate::create_flow::prompt_next_create_required(app).await;
            } else {
                advance_transition_field(app, &field, value);
                prompt_next_transition_field(app).await;
            }
        }
        _ => {}
    }
}

async fn prompt_next_transition_field(app: &mut App) {
    if !begin_next_field_prompt(app) {
        let Some(collect) = app.transition_collect.take() else {
            return;
        };
        execute_transition_with(app, collect.transition, collect.values).await;
        return;
    }
    if app.transition_field_user_search {
        refresh_transition_user_search(app, false).await;
    }
}

async fn apply_transition_field_pick(app: &mut App, idx: usize) {
    if idx >= app.transition_field_options.len() {
        return;
    }
    let Some(field) = app.transition_field_current.clone() else {
        return;
    };
    let (id, label) = app.transition_field_options[idx].clone();
    let value = field.value_from_choice(&id, &label);
    advance_transition_field(app, &field, value);
    prompt_next_transition_field(app).await;
}

/// Whether a transition user-field key is handled by the picker (not footer typing).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TransitionUserFieldKeyAction {
    Cancel,
    LoadMoreUsers,
    MoveUp,
    MoveDown,
    PickSelected,
    PickIndex(usize),
    PassToInput,
}

fn transition_user_field_key_action(
    key: &KeyEvent,
    has_options: bool,
) -> TransitionUserFieldKeyAction {
    match key.code {
        KeyCode::Esc => TransitionUserFieldKeyAction::Cancel,
        _ if load_more_users_key(key) => TransitionUserFieldKeyAction::LoadMoreUsers,
        KeyCode::Up | KeyCode::Char('k') if has_options => TransitionUserFieldKeyAction::MoveUp,
        KeyCode::Down | KeyCode::Char('j') if has_options => TransitionUserFieldKeyAction::MoveDown,
        KeyCode::Enter if has_options => TransitionUserFieldKeyAction::PickSelected,
        KeyCode::Char(n) if has_options && ('1'..='9').contains(&n) => {
            TransitionUserFieldKeyAction::PickIndex((n as u8 - b'1') as usize)
        }
        _ => TransitionUserFieldKeyAction::PassToInput,
    }
}

/// User field: picker keys (j/k, arrows, Ctrl+R, Enter) vs footer typing for filter text.
/// Returns `true` when the key was handled here (not passed to the input buffer).
async fn handle_transition_user_field_key(app: &mut App, key: &KeyEvent) -> bool {
    let has_options = !app.transition_field_options.is_empty();
    match transition_user_field_key_action(key, has_options) {
        TransitionUserFieldKeyAction::Cancel => {
            cancel_transition_collect(app);
            true
        }
        TransitionUserFieldKeyAction::LoadMoreUsers => {
            refresh_transition_user_search(app, true).await;
            true
        }
        TransitionUserFieldKeyAction::MoveUp => {
            app.transition_field_selected = app.transition_field_selected.saturating_sub(1);
            true
        }
        TransitionUserFieldKeyAction::MoveDown => {
            if app.transition_field_selected + 1 < app.transition_field_options.len() {
                app.transition_field_selected += 1;
            }
            true
        }
        TransitionUserFieldKeyAction::PickSelected => {
            apply_transition_field_pick(app, app.transition_field_selected).await;
            true
        }
        TransitionUserFieldKeyAction::PickIndex(idx) => {
            apply_transition_field_pick(app, idx).await;
            true
        }
        TransitionUserFieldKeyAction::PassToInput => false,
    }
}

async fn handle_transition_field_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Up | KeyCode::Char('k') => {
            app.transition_field_selected = app.transition_field_selected.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.transition_field_selected + 1 < app.transition_field_options.len() {
                app.transition_field_selected += 1;
            }
        }
        KeyCode::Enter => {
            apply_transition_field_pick(app, app.transition_field_selected).await;
        }
        KeyCode::Char(n) if ('1'..='9').contains(&n) => {
            let idx = (n as u8 - b'1') as usize;
            apply_transition_field_pick(app, idx).await;
        }
        KeyCode::Esc => cancel_transition_collect(app),
        _ => {}
    }
}

async fn execute_transition_with(
    app: &mut App,
    transition: WorkflowTransition,
    values: HashMap<String, serde_json::Value>,
) {
    let Some(sel) = app.selected_ticket() else {
        app.status.set_action_error("Select a ticket first");
        return;
    };
    let Some(base_url) = app.site_base_url(&sel.site) else {
        app.status
            .set_action_error(format!("Unknown site {:?} for ticket", sel.site));
        return;
    };

    app.loading = true;
    app.loading_message = Some(format!("Applying {}…", transition.label()));
    match app
        .jira
        .transition_issue(&base_url, &sel.key, &transition, &values)
        .await
    {
        Ok(()) => {
            cancel_transition_collect(app);
            app.refresh_all().await;
        }
        Err(e) if !e.field_errors.is_empty() => {
            let mut pending: Vec<TransitionField> = e
                .field_errors
                .iter()
                .map(|(id, _)| {
                    transition_fields::field_for_error_key(id, &transition.required_fields)
                })
                .collect();
            if let Some(base_url) = app.site_base_url(&sel.site) {
                let mut tmp = transition.clone();
                tmp.required_fields = pending.clone();
                let pk = crate::api::types::project_key_from_issue_key(&sel.key);
                api::enrich_transition_fields(&app.jira, &base_url, Some(pk), &mut tmp).await;
                pending = tmp.required_fields;
            }
            app.transition_collect = Some(TransitionCollect {
                transition,
                values,
                pending,
            });
            app.status.set_action_error(e.message);
            let _ = begin_next_field_prompt(app);
        }
        Err(e) => {
            cancel_transition_collect(app);
            app.status.set_action_error(e.message);
        }
    }
    app.loading = false;
    app.loading_message = None;
}

async fn apply_transition(app: &mut App, idx: usize) {
    if idx >= app.transition_options.len() {
        return;
    }
    let mut transition = app.transition_options[idx].clone();
    let Some(sel) = app.selected_ticket() else {
        app.status.set_action_error("Select a ticket first");
        return;
    };
    let Some(base_url) = app.site_base_url(&sel.site) else {
        app.status
            .set_action_error(format!("Unknown site {:?} for ticket", sel.site));
        return;
    };
    app.showing_transitions = false;

    app.loading = true;
    app.loading_message = Some("Loading transition fields…".into());
    let (detail, _) = tokio::join!(
        async {
            if transition_fields::transition_needs_detail_fetch(&transition) {
                app.jira
                    .get_transition_detail(&base_url, &sel.key, &transition.id)
                    .await
                    .ok()
            } else {
                None
            }
        },
        app.jira.warm_site_field_catalogs(&base_url)
    );
    if let Some(detail) = detail {
        transition = detail;
    }

    if transition.required_fields.is_empty() {
        if let Some(res) = transition_fields::infer_resolution_if_done_transition(
            &transition.name,
            &transition.to_status,
        ) {
            transition.required_fields.push(res);
        }
    }
    let pk = crate::api::types::project_key_from_issue_key(&sel.key);
    api::enrich_transition_fields(&app.jira, &base_url, Some(pk), &mut transition).await;
    app.loading = false;
    app.loading_message = None;

    if transition.required_fields.is_empty() {
        execute_transition_with(app, transition, HashMap::new()).await;
        return;
    }

    app.transition_collect = Some(TransitionCollect {
        pending: transition.required_fields.clone(),
        transition,
        values: HashMap::new(),
    });
    prompt_next_transition_field(app).await;
}

async fn handle_normal_key(app: &mut App, code: KeyCode) -> bool {
    match code {
        KeyCode::Char('q') => return true,
        KeyCode::Char('r') => {
            app.refresh().await;
        }
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

fn parse_due_date_input(buffer: &str) -> Result<Option<chrono::NaiveDate>, String> {
    let trimmed = buffer.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    chrono::NaiveDate::parse_from_str(trimmed, "%Y-%m-%d")
        .map(Some)
        .map_err(|_| "Due date must be YYYY-MM-DD (empty to clear)".into())
}

async fn watch_ticket(app: &mut App) {
    let Some(sel) = app.selected_ticket() else {
        app.status.set_action_error("Select a ticket first");
        return;
    };
    let Some(base_url) = app.site_base_url(&sel.site) else {
        app.status.set_action_error("Unknown site for ticket");
        return;
    };
    match app.jira.watch_issue(&base_url, &sel.key).await {
        Ok(()) => {
            app.status
                .set_action_notice(format!("Watching {}", sel.key));
            app.refresh_all().await;
        }
        Err(e) => app.status.set_action_error(e),
    }
}

async fn unwatch_ticket(app: &mut App) {
    let Some(sel) = app.selected_ticket() else {
        app.status.set_action_error("Select a ticket first");
        return;
    };
    let Some(base_url) = app.site_base_url(&sel.site) else {
        app.status.set_action_error("Unknown site for ticket");
        return;
    };
    match app.jira.unwatch_issue(&base_url, &sel.key).await {
        Ok(()) => {
            app.status
                .set_action_notice(format!("Unwatched {}", sel.key));
            app.refresh_all().await;
        }
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

async fn start_status_picker(app: &mut App) {
    let Some(sel) = app.selected_ticket() else {
        app.status
            .set_action_error("Select a ticket to change status");
        return;
    };
    let Some(base_url) = app.site_base_url(&sel.site) else {
        app.status.set_action_error(format!(
            "Unknown site {:?} in config — cannot change status for {}",
            sel.site, sel.key
        ));
        return;
    };
    let current_status = app
        .selected_ticket_entry()
        .map(|t| t.status.clone())
        .unwrap_or_else(|| "Unknown".into());

    app.loading = true;
    app.loading_message = Some("Loading workflow transitions…".into());
    let (result, _) = tokio::join!(
        app.jira.get_workflow_transitions(&base_url, &sel.key),
        app.jira.warm_site_field_catalogs(&base_url)
    );
    app.loading = false;
    app.loading_message = None;

    match result {
        Ok(options) if !options.is_empty() => {
            app.transition_options = options;
            app.transition_selected = 0;
            app.showing_transitions = true;
        }
        Ok(_) => {
            app.status.set_action_error(format!(
                "No workflow transitions for {} (status: \"{}\"). The issue may be closed, or your role cannot move it in this workflow.",
                sel.key, current_status
            ));
        }
        Err(e) => app.status.set_action_error(e),
    }
}

async fn try_switch_custom_key(app: &mut App, key: u8) {
    if let Some((_, index)) = app
        .config
        .custom_view_keys()
        .into_iter()
        .find(|(k, _)| *k == key)
    {
        app.switch_to_custom(index).await;
    }
}

#[cfg(test)]
mod key_tests {
    use super::{
        active_mention_query, load_more_users_key, mentions_enabled,
        transition_user_field_key_action, TransitionUserFieldKeyAction,
    };
    use crate::app::InputMode;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

    fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn load_more_users_accepts_modifier_r() {
        assert!(load_more_users_key(&key(
            KeyCode::Char('r'),
            KeyModifiers::CONTROL
        )));
        assert!(load_more_users_key(&key(
            KeyCode::Char('r'),
            KeyModifiers::SUPER
        )));
        assert!(load_more_users_key(&key(
            KeyCode::Char('r'),
            KeyModifiers::META
        )));
        assert!(!load_more_users_key(&key(
            KeyCode::Char('r'),
            KeyModifiers::empty()
        )));
        assert!(!load_more_users_key(&key(
            KeyCode::Char('R'),
            KeyModifiers::SHIFT
        )));
    }

    #[test]
    fn mentions_enabled_only_for_comment_and_description() {
        assert!(mentions_enabled(InputMode::Comment));
        assert!(mentions_enabled(InputMode::EditDescription));
        assert!(!mentions_enabled(InputMode::None));
        assert!(!mentions_enabled(InputMode::TransitionField));
        assert!(!mentions_enabled(InputMode::EditSummary));
    }

    #[test]
    fn transition_user_field_plain_r_passes_to_input() {
        assert_eq!(
            transition_user_field_key_action(&key(KeyCode::Char('r'), KeyModifiers::empty()), true),
            TransitionUserFieldKeyAction::PassToInput
        );
        assert_eq!(
            transition_user_field_key_action(&key(KeyCode::Char('R'), KeyModifiers::SHIFT), true),
            TransitionUserFieldKeyAction::PassToInput
        );
    }

    #[test]
    fn transition_user_field_modifier_r_loads_more() {
        assert_eq!(
            transition_user_field_key_action(
                &key(KeyCode::Char('r'), KeyModifiers::CONTROL),
                false
            ),
            TransitionUserFieldKeyAction::LoadMoreUsers
        );
    }

    #[test]
    fn transition_user_field_j_k_only_when_options() {
        assert_eq!(
            transition_user_field_key_action(&key(KeyCode::Char('j'), KeyModifiers::empty()), true),
            TransitionUserFieldKeyAction::MoveDown
        );
        assert_eq!(
            transition_user_field_key_action(&key(KeyCode::Char('k'), KeyModifiers::empty()), true),
            TransitionUserFieldKeyAction::MoveUp
        );
        assert_eq!(
            transition_user_field_key_action(
                &key(KeyCode::Char('j'), KeyModifiers::empty()),
                false
            ),
            TransitionUserFieldKeyAction::PassToInput
        );
    }

    #[test]
    fn transition_user_field_numeric_pick() {
        assert_eq!(
            transition_user_field_key_action(&key(KeyCode::Char('3'), KeyModifiers::empty()), true),
            TransitionUserFieldKeyAction::PickIndex(2)
        );
        assert_eq!(
            transition_user_field_key_action(
                &key(KeyCode::Char('3'), KeyModifiers::empty()),
                false
            ),
            TransitionUserFieldKeyAction::PassToInput
        );
    }

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
