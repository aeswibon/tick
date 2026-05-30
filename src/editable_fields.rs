//! Config-driven custom field editing from the issue detail pane.

use crossterm::event::KeyCode;
use serde_json::{json, Value};

use crate::api::transition_fields::{TransitionField, TransitionFieldKind};
use crate::app::{App, InputMode};
use crate::config::{EditableFieldConfig, EditableFieldKind};

pub fn start_editable_field_flow(app: &mut App) {
    let fields = &app.config.detail.editable_fields;
    if fields.is_empty() {
        app.status.set_action_error(
            "No [[detail.editable_fields]] in config — add fields to edit custom values",
        );
        return;
    }
    if fields.len() == 1 {
        begin_edit_field(app, fields[0].clone());
        return;
    }
    app.editable_field_picker_selected = 0;
    app.showing_editable_field_picker = true;
}

pub fn handle_editable_field_picker_key(app: &mut App, code: KeyCode) {
    let fields = &app.config.detail.editable_fields;
    match code {
        KeyCode::Up | KeyCode::Char('k') => {
            app.editable_field_picker_selected =
                app.editable_field_picker_selected.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.editable_field_picker_selected + 1 < fields.len() {
                app.editable_field_picker_selected += 1;
            }
        }
        KeyCode::Enter => {
            let idx = app.editable_field_picker_selected;
            if idx < fields.len() {
                let field = fields[idx].clone();
                app.showing_editable_field_picker = false;
                begin_edit_field(app, field);
            }
        }
        KeyCode::Char(n) if ('1'..='9').contains(&n) => {
            let idx = (n as u8 - b'1') as usize;
            if idx < fields.len() {
                let field = fields[idx].clone();
                app.showing_editable_field_picker = false;
                begin_edit_field(app, field);
            }
        }
        KeyCode::Esc => cancel_custom_field_edit(app),
        _ => {}
    }
}

fn begin_edit_field(app: &mut App, field: EditableFieldConfig) {
    let kind = match field.parsed_kind() {
        Ok(k) => k,
        Err(e) => {
            app.status.set_action_error(e);
            return;
        }
    };
    let current = app
        .selected_ticket_entry()
        .and_then(|t| t.custom_fields.get(&field.id).cloned())
        .unwrap_or_default();

    app.custom_field_editing = Some(field.clone());

    match kind {
        EditableFieldKind::Text => {
            app.input_mode = InputMode::EditCustomField;
            app.input_buffer = current;
        }
        EditableFieldKind::Select => {
            app.custom_field_select_options = field.options.clone();
            app.custom_field_select_selected = field
                .options
                .iter()
                .position(|o| o == &current)
                .unwrap_or(0);
            app.showing_custom_field_select = true;
        }
        EditableFieldKind::User => {
            let transition_field = TransitionField {
                id: field.id.clone(),
                name: field.display_label(),
                field_type: "user".into(),
                system: String::new(),
                kind: TransitionFieldKind::User,
                options: Vec::new(),
            };
            app.transition_field_heading = transition_field.name.clone();
            app.transition_field_current = Some(transition_field);
            app.showing_transition_field = true;
            app.transition_field_text_mode = true;
            app.transition_field_user_search = true;
            app.transition_field_options.clear();
            app.transition_field_selected = 0;
            app.input_mode = InputMode::TransitionField;
            app.input_buffer.clear();
        }
    }
}

pub async fn submit_custom_field_text(app: &mut App, buffer: String) {
    let Some(field) = app.custom_field_editing.clone() else {
        return;
    };
    let trimmed = buffer.trim();
    let value = if trimmed.is_empty() {
        Value::Null
    } else {
        json!({ "value": trimmed })
    };
    apply_custom_field_value(app, &field, value).await;
}

pub async fn apply_custom_field_select(app: &mut App, idx: usize) {
    let Some(field) = app.custom_field_editing.clone() else {
        return;
    };
    if idx >= app.custom_field_select_options.len() {
        return;
    }
    let option = app.custom_field_select_options[idx].clone();
    let value = json!({ "value": option });
    apply_custom_field_value(app, &field, value).await;
}

pub async fn apply_custom_field_user_pick(app: &mut App, account_id: String) {
    let Some(field) = app.custom_field_editing.take() else {
        return;
    };
    let value = json!({ "accountId": account_id });
    apply_custom_field_value(app, &field, value).await;
}

async fn apply_custom_field_value(app: &mut App, field: &EditableFieldConfig, value: Value) {
    let Some(sel) = app.selected_ticket() else {
        cancel_custom_field_edit(app);
        return;
    };
    let Some(base_url) = app.site_base_url(&sel.site) else {
        app.status.set_action_error("Unknown site for ticket");
        cancel_custom_field_edit(app);
        return;
    };
    match app
        .jira
        .update_issue_field(&base_url, &sel.key, &field.id, value)
        .await
    {
        Ok(()) => {
            cancel_custom_field_edit(app);
            app.status
                .set_action_notice(format!("Updated {}", field.display_label()));
            app.refresh().await;
        }
        Err(e) => {
            app.status.set_action_error(e);
            cancel_custom_field_edit(app);
        }
    }
}

pub fn cancel_custom_field_edit(app: &mut App) {
    app.custom_field_editing = None;
    app.showing_editable_field_picker = false;
    app.showing_custom_field_select = false;
    app.custom_field_select_options.clear();
    if app.input_mode == InputMode::EditCustomField || app.input_mode == InputMode::TransitionField
    {
        app.input_mode = InputMode::None;
        app.input_buffer.clear();
    }
    if app.showing_transition_field && app.transition_collect.is_none() {
        app.showing_transition_field = false;
        app.transition_field_text_mode = false;
        app.transition_field_user_search = false;
        app.transition_field_current = None;
        app.transition_field_options.clear();
    }
}

pub async fn handle_custom_field_select_key(app: &mut App, code: KeyCode) {
    let len = app.custom_field_select_options.len();
    match code {
        KeyCode::Up | KeyCode::Char('k') => {
            app.custom_field_select_selected = app.custom_field_select_selected.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.custom_field_select_selected + 1 < len {
                app.custom_field_select_selected += 1;
            }
        }
        KeyCode::Enter => {
            let idx = app.custom_field_select_selected;
            apply_custom_field_select(app, idx).await;
        }
        KeyCode::Char(n) if ('1'..='9').contains(&n) => {
            let idx = (n as u8 - b'1') as usize;
            apply_custom_field_select(app, idx).await;
        }
        KeyCode::Esc => cancel_custom_field_edit(app),
        _ => {}
    }
}
