use crossterm::event::{KeyCode, KeyEvent};

use crate::api::transition_fields::TransitionFieldKind;
use crate::app::{App, InputMode};

use super::detail_actions::parse_due_date_input;
use super::load_more_users_key;
use super::transitions::{
    advance_transition_field, apply_transition_field_pick, prompt_next_transition_field,
};

pub(crate) fn clear_mention_picker(app: &mut App) {
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

pub(crate) async fn refresh_mention_catalog(app: &mut App, force: bool) -> Result<(), String> {
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

pub(crate) async fn refresh_mention_picker(app: &mut App) {
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

pub(crate) fn mentions_enabled(mode: InputMode) -> bool {
    matches!(mode, InputMode::Comment | InputMode::EditDescription)
}

pub(crate) async fn handle_mention_picker_key(app: &mut App, key: &KeyEvent) {
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

pub(crate) async fn start_open_ticket(app: &mut App) {
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

pub(crate) async fn submit_open_ticket(app: &mut App) {
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

pub(crate) async fn submit_input(app: &mut App) {
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
        InputMode::EditCustomField => {
            crate::editable_fields::submit_custom_field_text(app, buffer).await;
            return;
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
        | InputMode::TemplateEditIssueType
        | InputMode::TemplateEditDescription
        | InputMode::TemplateEditLabels
        | InputMode::BulkEditLabels
        | InputMode::GlobalSearchQuery => {
            return;
        }
        InputMode::OpenTicket | InputMode::None => return,
    };

    match result {
        Ok(()) => app.refresh().await,
        Err(e) => app.status.set_action_error(e),
    }
}
