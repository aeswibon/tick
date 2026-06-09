//! Config-driven custom field editing from the issue detail pane.

use crossterm::event::KeyCode;
use serde_json::{json, Value};

use crate::api::fields::{select_options_from_transition_field, tick_type_from_transition_field};
use crate::api::transition_fields::{TransitionField, TransitionFieldKind, BOOLEAN_OPTIONS};
use crate::app::{App, InputMode};
use crate::config::{EditableFieldConfig, EditableFieldKind};

pub async fn start_editable_field_flow(app: &mut App) {
    let fields = app.config.detail.editable_fields.clone();
    if fields.is_empty() {
        app.status.set_action_error(
            "No [[detail.editable_fields]] in config — add fields to edit custom values",
        );
        return;
    }
    if fields.len() == 1 {
        begin_edit_field(app, fields[0].clone()).await;
        return;
    }
    app.editable_field_picker_selected = 0;
    app.showing_editable_field_picker = true;
}

pub async fn handle_editable_field_picker_key(app: &mut App, code: KeyCode) {
    let field_count = app.config.detail.editable_fields.len();
    match code {
        KeyCode::Up | KeyCode::Char('k') => {
            app.editable_field_picker_selected =
                app.editable_field_picker_selected.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.editable_field_picker_selected + 1 < field_count {
                app.editable_field_picker_selected += 1;
            }
        }
        KeyCode::Enter => {
            let idx = app.editable_field_picker_selected;
            if idx < field_count {
                let field = app.config.detail.editable_fields[idx].clone();
                app.showing_editable_field_picker = false;
                begin_edit_field(app, field).await;
            }
        }
        KeyCode::Char(n) if ('1'..='9').contains(&n) => {
            let idx = (n as u8 - b'1') as usize;
            if idx < field_count {
                let field = app.config.detail.editable_fields[idx].clone();
                app.showing_editable_field_picker = false;
                begin_edit_field(app, field).await;
            }
        }
        KeyCode::Esc => cancel_custom_field_edit(app),
        _ => {}
    }
}

async fn begin_edit_field(app: &mut App, field: EditableFieldConfig) {
    let kind = match field.parsed_kind() {
        Ok(k) => k,
        Err(e) => {
            app.status.set_action_error(e);
            return;
        }
    };

    let needs_meta = matches!(kind, EditableFieldKind::Auto)
        || (matches!(kind, EditableFieldKind::Select) && field.options.is_empty())
        || (matches!(kind, EditableFieldKind::MultiSelect) && field.options.is_empty());

    let mut resolved_kind = kind;
    let mut options = field.options.clone();
    let mut meta: Option<TransitionField> = None;

    if needs_meta {
        let Some(sel) = app.selected_ticket() else {
            app.status.set_action_error("Select a ticket first");
            return;
        };
        let Some(base_url) = app.site_base_url(&sel.site) else {
            app.status.set_action_error("Unknown site for ticket");
            return;
        };
        match app
            .jira
            .fetch_editmeta_field(&base_url, &sel.key, &field.id)
            .await
        {
            Ok(Some(tf)) => {
                if matches!(kind, EditableFieldKind::Auto) {
                    resolved_kind = match tick_type_from_transition_field(&tf) {
                        "user" => EditableFieldKind::User,
                        "select" => EditableFieldKind::Select,
                        "text" => EditableFieldKind::Text,
                        "number" => EditableFieldKind::Number,
                        "date" => EditableFieldKind::Date,
                        "datetime" => EditableFieldKind::DateTime,
                        "boolean" => EditableFieldKind::Boolean,
                        "multiselect" => EditableFieldKind::MultiSelect,
                        _ => {
                            app.status.set_action_error(format!(
                                "{} cannot be edited from the detail pane",
                                field.display_label()
                            ));
                            return;
                        }
                    };
                }
                if matches!(resolved_kind, EditableFieldKind::Select) {
                    options = select_options_from_transition_field(&tf);
                    if options.is_empty() {
                        app.status.set_action_error(format!(
                            "No options for {} on this issue",
                            field.display_label()
                        ));
                        return;
                    }
                }
                if matches!(resolved_kind, EditableFieldKind::MultiSelect) && tf.options.is_empty()
                {
                    app.status.set_action_error(format!(
                        "No options for {} on this issue",
                        field.display_label()
                    ));
                    return;
                }
                meta = Some(tf);
            }
            Ok(None) => {
                app.status.set_action_error(format!(
                    "{} is not editable on this issue",
                    field.display_label()
                ));
                return;
            }
            Err(e) => {
                app.status.set_action_error(e);
                return;
            }
        }
    }

    let current = app
        .selected_ticket_entry()
        .and_then(|t| t.custom_fields.get(&field.id).cloned())
        .unwrap_or_default();

    app.custom_field_editing = Some(field.clone());
    app.custom_field_meta = meta;

    match resolved_kind {
        EditableFieldKind::Text => {
            app.input_mode = InputMode::EditCustomField;
            app.input_buffer = current;
        }
        EditableFieldKind::Number | EditableFieldKind::Date | EditableFieldKind::DateTime => {
            if app.custom_field_meta.is_none() {
                app.custom_field_meta = Some(synthetic_field(
                    &field,
                    match resolved_kind {
                        EditableFieldKind::Number => TransitionFieldKind::Number,
                        EditableFieldKind::Date => TransitionFieldKind::Date,
                        EditableFieldKind::DateTime => TransitionFieldKind::DateTime,
                        _ => unreachable!(),
                    },
                ));
            }
            app.input_mode = InputMode::EditCustomField;
            app.input_buffer = if current == "-" {
                String::new()
            } else {
                current
            };
        }
        EditableFieldKind::Select | EditableFieldKind::Boolean => {
            let select_options =
                if matches!(resolved_kind, EditableFieldKind::Boolean) && options.is_empty() {
                    boolean_select_options()
                } else {
                    options
                };
            if app.custom_field_meta.is_none()
                && matches!(resolved_kind, EditableFieldKind::Boolean)
            {
                app.custom_field_meta = Some(synthetic_boolean_field(&field));
            }
            start_select_editor(
                app,
                select_options,
                &current,
                matches!(resolved_kind, EditableFieldKind::Boolean),
            );
        }
        EditableFieldKind::MultiSelect => {
            let multi_options = multi_options_for_field(&field, app.custom_field_meta.as_ref());
            app.custom_field_multi_options = multi_options;
            app.custom_field_multi_picked =
                prefill_multi_picks(&current, &app.custom_field_multi_options);
            app.custom_field_select_selected = 0;
            app.showing_custom_field_multi = true;
        }
        EditableFieldKind::User => {
            let transition_field =
                app.custom_field_meta
                    .clone()
                    .unwrap_or_else(|| TransitionField {
                        id: field.id.clone(),
                        name: field.display_label(),
                        field_type: "user".into(),
                        system: String::new(),
                        kind: TransitionFieldKind::User,
                        options: Vec::new(),
                    });
            start_user_field_editor(app, transition_field);
        }
        EditableFieldKind::Auto => unreachable!("resolved before match"),
    }
}

fn synthetic_boolean_field(field: &EditableFieldConfig) -> TransitionField {
    TransitionField {
        id: field.id.clone(),
        name: field.display_label(),
        field_type: "boolean".into(),
        system: String::new(),
        kind: TransitionFieldKind::Boolean,
        options: BOOLEAN_OPTIONS
            .iter()
            .map(|(id, label)| (id.to_string(), label.to_string()))
            .collect(),
    }
}

fn boolean_select_options() -> Vec<String> {
    BOOLEAN_OPTIONS
        .iter()
        .map(|(_, label)| label.to_string())
        .collect()
}

fn start_select_editor(app: &mut App, options: Vec<String>, current: &str, is_boolean: bool) {
    app.custom_field_select_options = options;
    app.custom_field_select_selected = if is_boolean {
        prefill_boolean_select(current, &app.custom_field_select_options)
    } else {
        app.custom_field_select_options
            .iter()
            .position(|o| o == current)
            .unwrap_or(0)
    };
    app.showing_custom_field_select = true;
}

fn prefill_boolean_select(current: &str, options: &[String]) -> usize {
    let label = match current {
        "true" | "Yes" => "Yes",
        "false" | "No" => "No",
        "-" => return 0,
        other => other,
    };
    options.iter().position(|o| o == label).unwrap_or(0)
}

fn synthetic_field(field: &EditableFieldConfig, kind: TransitionFieldKind) -> TransitionField {
    let field_type = match kind {
        TransitionFieldKind::Number => "number",
        TransitionFieldKind::Date => "date",
        TransitionFieldKind::DateTime => "datetime",
        _ => "string",
    };
    TransitionField {
        id: field.id.clone(),
        name: field.display_label(),
        field_type: field_type.into(),
        system: String::new(),
        kind,
        options: Vec::new(),
    }
}

fn multi_options_for_field(
    field: &EditableFieldConfig,
    meta: Option<&TransitionField>,
) -> Vec<(String, String)> {
    if let Some(tf) = meta {
        return tf.options.clone();
    }
    field
        .options
        .iter()
        .map(|o| (o.clone(), o.clone()))
        .collect()
}

fn prefill_multi_picks(current: &str, options: &[(String, String)]) -> Vec<bool> {
    if current.is_empty() || current == "-" {
        return vec![false; options.len()];
    }
    let selected: Vec<&str> = current.split(", ").map(str::trim).collect();
    options
        .iter()
        .map(|(_, label)| selected.contains(&label.as_str()))
        .collect()
}

fn start_user_field_editor(app: &mut App, transition_field: TransitionField) {
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

pub async fn submit_custom_field_text(app: &mut App, buffer: String) {
    let Some(field) = app.custom_field_editing.clone() else {
        return;
    };
    let value = if buffer.trim().is_empty() {
        Value::Null
    } else if let Some(meta) = app.custom_field_meta.as_ref() {
        match meta.value_from_text(&buffer) {
            Ok(v) => v,
            Err(e) => {
                app.status.set_action_error(e);
                return;
            }
        }
    } else {
        json!({ "value": buffer.trim() })
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
    let value = if let Some(meta) = app.custom_field_meta.as_ref() {
        if let Some((id, _)) = meta.options.iter().find(|(_, label)| label == &option) {
            meta.value_from_choice(id, &option)
        } else if meta.kind == TransitionFieldKind::Boolean {
            boolean_value_from_label(&option)
        } else {
            json!({ "value": option })
        }
    } else if field.parsed_kind() == Ok(EditableFieldKind::Boolean) {
        boolean_value_from_label(&option)
    } else {
        json!({ "value": option })
    };
    apply_custom_field_value(app, &field, value).await;
}

fn boolean_value_from_label(label: &str) -> Value {
    match label {
        "Yes" => json!(true),
        "No" => json!(false),
        other => json!({ "value": other }),
    }
}

pub async fn apply_custom_field_multi(app: &mut App) {
    let Some(field) = app.custom_field_editing.clone() else {
        return;
    };
    let picks: Vec<_> = app
        .custom_field_multi_options
        .iter()
        .enumerate()
        .filter(|(i, _)| {
            app.custom_field_multi_picked
                .get(*i)
                .copied()
                .unwrap_or(false)
        })
        .map(|(_, pair)| pair.clone())
        .collect();
    let value = if picks.is_empty() {
        Value::Null
    } else if let Some(meta) = app.custom_field_meta.as_ref() {
        meta.value_from_multi_choices(&picks)
    } else {
        json!(picks
            .into_iter()
            .map(|(_, label)| json!({ "value": label }))
            .collect::<Vec<_>>())
    };
    apply_custom_field_value(app, &field, value).await;
}

pub async fn apply_custom_field_user_pick(app: &mut App, account_id: String) {
    let Some(field) = app.custom_field_editing.clone() else {
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
    app.custom_field_meta = None;
    app.showing_editable_field_picker = false;
    app.showing_custom_field_select = false;
    app.custom_field_select_options.clear();
    app.showing_custom_field_multi = false;
    app.custom_field_multi_options.clear();
    app.custom_field_multi_picked.clear();
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

pub async fn handle_custom_field_multi_key(app: &mut App, code: KeyCode) {
    let len = app.custom_field_multi_options.len();
    match code {
        KeyCode::Up | KeyCode::Char('k') => {
            app.custom_field_select_selected = app.custom_field_select_selected.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.custom_field_select_selected + 1 < len {
                app.custom_field_select_selected += 1;
            }
        }
        KeyCode::Char(' ') => {
            let i = app.custom_field_select_selected;
            if let Some(slot) = app.custom_field_multi_picked.get_mut(i) {
                *slot = !*slot;
            }
        }
        KeyCode::Enter => apply_custom_field_multi(app).await,
        KeyCode::Esc => cancel_custom_field_edit(app),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefill_multi_picks_from_display() {
        let options = vec![
            ("1".into(), "Alpha".into()),
            ("2".into(), "Beta".into()),
            ("3".into(), "Gamma".into()),
        ];
        let picked = prefill_multi_picks("Alpha, Gamma", &options);
        assert_eq!(picked, vec![true, false, true]);
    }

    #[test]
    fn prefill_boolean_select_maps_true_false() {
        let options = boolean_select_options();
        assert_eq!(prefill_boolean_select("true", &options), 0);
        assert_eq!(prefill_boolean_select("false", &options), 1);
        assert_eq!(prefill_boolean_select("Yes", &options), 0);
    }

    #[test]
    fn tick_type_maps_number_date_multiselect() {
        let number = TransitionField {
            id: "customfield_1".into(),
            name: "Points".into(),
            field_type: "number".into(),
            system: String::new(),
            kind: TransitionFieldKind::Number,
            options: vec![],
        };
        assert_eq!(tick_type_from_transition_field(&number), "number");

        let date = TransitionField {
            id: "customfield_2".into(),
            name: "Start".into(),
            field_type: "date".into(),
            system: String::new(),
            kind: TransitionFieldKind::Date,
            options: vec![],
        };
        assert_eq!(tick_type_from_transition_field(&date), "date");

        let datetime = TransitionField {
            id: "customfield_4".into(),
            name: "Due at".into(),
            field_type: "datetime".into(),
            system: String::new(),
            kind: TransitionFieldKind::DateTime,
            options: vec![],
        };
        assert_eq!(tick_type_from_transition_field(&datetime), "datetime");

        let boolean = TransitionField {
            id: "customfield_5".into(),
            name: "Approved".into(),
            field_type: "boolean".into(),
            system: String::new(),
            kind: TransitionFieldKind::Boolean,
            options: vec![("true".into(), "Yes".into()), ("false".into(), "No".into())],
        };
        assert_eq!(tick_type_from_transition_field(&boolean), "boolean");

        let multi = TransitionField {
            id: "customfield_3".into(),
            name: "Tags".into(),
            field_type: "array".into(),
            system: String::new(),
            kind: TransitionFieldKind::MultiPicker,
            options: vec![("1".into(), "A".into())],
        };
        assert_eq!(tick_type_from_transition_field(&multi), "multiselect");
    }
}
