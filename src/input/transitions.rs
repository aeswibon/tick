use crossterm::event::{KeyCode, KeyEvent};

use std::collections::HashMap;

use crate::api::assignable_users;
use crate::api::transition_fields::{self, TransitionField, TransitionFieldKind};
use crate::api::{self, types::WorkflowTransition};
use crate::app::{App, InputMode, TransitionCollect};

use super::load_more_users_key;

pub(crate) async fn handle_transition_key(app: &mut App, code: KeyCode) {
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
        KeyCode::Esc => {
            app.showing_transitions = false;
            app.bulk_action = None;
        }
        _ => {}
    }
}

pub(crate) async fn handle_sprint_key(app: &mut App, code: KeyCode) {
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

pub(crate) async fn apply_sprint_move(app: &mut App, idx: usize) {
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
            app.refresh().await;
        }
        Err(e) => {
            app.status.set_action_error(e);
            app.showing_sprints = false;
        }
    }
}

pub(crate) async fn handle_priority_key(app: &mut App, code: KeyCode) {
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

pub(crate) async fn apply_priority(app: &mut App, idx: usize) {
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
            app.refresh().await;
        }
        Err(e) => {
            app.status.set_action_error(e);
            app.showing_priorities = false;
        }
    }
}

pub(crate) async fn refresh_transition_user_search(app: &mut App, force_refresh: bool) {
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

pub(crate) fn cancel_transition_collect(app: &mut App) {
    if app.custom_field_editing.is_some() && app.transition_collect.is_none() {
        crate::editable_fields::cancel_custom_field_edit(app);
        return;
    }
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

pub(crate) fn advance_transition_field(
    app: &mut App,
    field: &TransitionField,
    value: serde_json::Value,
) {
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
pub(crate) fn begin_next_field_prompt(app: &mut App) -> bool {
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

pub(crate) async fn handle_transition_multi_field_key(app: &mut App, code: KeyCode) {
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

pub(crate) async fn prompt_next_transition_field(app: &mut App) {
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

pub(crate) async fn apply_transition_field_pick(app: &mut App, idx: usize) {
    if idx >= app.transition_field_options.len() {
        return;
    }
    if app.custom_field_editing.is_some() {
        let (account_id, _) = app.transition_field_options[idx].clone();
        crate::editable_fields::apply_custom_field_user_pick(app, account_id).await;
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
pub(crate) enum TransitionUserFieldKeyAction {
    Cancel,
    LoadMoreUsers,
    MoveUp,
    MoveDown,
    PickSelected,
    PickIndex(usize),
    PassToInput,
}

pub(crate) fn transition_user_field_key_action(
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
pub(crate) async fn handle_transition_user_field_key(app: &mut App, key: &KeyEvent) -> bool {
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

pub(crate) async fn handle_transition_field_key(app: &mut App, code: KeyCode) {
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

pub(crate) async fn execute_transition_with(
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
            app.refresh().await;
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

pub(crate) async fn apply_transition(app: &mut App, idx: usize) {
    if idx >= app.transition_options.len() {
        return;
    }
    if let Some(crate::bulk::BulkAction::Transition { site, keys }) = app.bulk_action.take() {
        let transition = app.transition_options[idx].clone();
        app.showing_transitions = false;
        crate::bulk::apply_bulk_transition_by_name(app, &site, &keys, &transition).await;
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
