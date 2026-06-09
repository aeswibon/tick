//! Inline field edits during duplicate review (`C`).

use chrono::NaiveDate;

use crate::api::create::CreateDraft;
use crate::input::parse_due_date_input;
use crate::template_export::{TemplateFieldId, TemplateFieldRow};

pub fn duplicate_field_is_editable(id: &TemplateFieldId) -> bool {
    !matches!(id, TemplateFieldId::Sprint)
}

pub fn draft_text_for_row(
    draft: &CreateDraft,
    id: &TemplateFieldId,
    sprint_field: Option<&str>,
) -> String {
    match id {
        TemplateFieldId::Summary => draft.summary.clone(),
        TemplateFieldId::Description => draft.description.clone(),
        TemplateFieldId::Labels => draft.labels.join(", "),
        TemplateFieldId::Priority => draft.priority_name.clone(),
        TemplateFieldId::Assignee => draft
            .assignee_account_id
            .as_deref()
            .map(|id| format!("accountId {id}"))
            .unwrap_or_default(),
        TemplateFieldId::Parent => draft.parent_key.clone().unwrap_or_default(),
        TemplateFieldId::DueDate => draft
            .due_date
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_default(),
        TemplateFieldId::Sprint => sprint_preview_text(draft, sprint_field),
        TemplateFieldId::Custom(cf) => custom_field_edit_text(draft, cf),
    }
}

pub fn duplicate_field_uses_multiline(id: &TemplateFieldId) -> bool {
    matches!(id, TemplateFieldId::Description)
}

/// Apply footer text to the draft; returns updated preview for the picker row.
pub fn apply_duplicate_row_edit(
    draft: &mut CreateDraft,
    id: &TemplateFieldId,
    _sprint_field: Option<&str>,
    text: &str,
) -> Result<String, String> {
    match id {
        TemplateFieldId::Summary => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                return Err("Summary cannot be empty".into());
            }
            draft.summary = trimmed.to_string();
            Ok(preview_line(trimmed, 72))
        }
        TemplateFieldId::Description => {
            let body = if duplicate_field_uses_multiline(id) {
                text.trim_end().to_string()
            } else {
                text.trim().to_string()
            };
            draft.description = body.clone();
            draft.description_adf = None;
            Ok(preview_line(&body, 72))
        }
        TemplateFieldId::Labels => {
            draft.labels = crate::app::parse_labels_input(text);
            Ok(if draft.labels.is_empty() {
                "(empty)".into()
            } else {
                draft.labels.join(", ")
            })
        }
        TemplateFieldId::Priority => {
            let trimmed = text.trim();
            draft.priority_name = trimmed.to_string();
            draft.priority_id = None;
            Ok(if trimmed.is_empty() {
                "(empty)".into()
            } else {
                trimmed.to_string()
            })
        }
        TemplateFieldId::Assignee => {
            let trimmed = text.trim();
            draft.assignee_account_id = if trimmed.is_empty() {
                None
            } else {
                let id = trimmed.strip_prefix("accountId ").unwrap_or(trimmed).trim();
                if id.is_empty() {
                    None
                } else {
                    Some(id.to_string())
                }
            };
            Ok(draft
                .assignee_account_id
                .as_deref()
                .map(|id| format!("accountId {id}"))
                .unwrap_or_else(|| "(empty)".to_string()))
        }
        TemplateFieldId::Parent => {
            let trimmed = text.trim();
            draft.parent_key = if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_uppercase())
            };
            Ok(draft
                .parent_key
                .clone()
                .unwrap_or_else(|| "(empty)".to_string()))
        }
        TemplateFieldId::DueDate => {
            let due = parse_due_date_input(text)?;
            draft.due_date = due;
            Ok(due
                .map(|d| d.format("%Y-%m-%d").to_string())
                .unwrap_or_else(|| "(empty)".to_string()))
        }
        TemplateFieldId::Sprint => Err("Sprint cannot be edited here — change after create".into()),
        TemplateFieldId::Custom(cf) => apply_custom_field_edit(draft, cf, text),
    }
    .map(|preview| {
        if preview == "-" {
            "(empty)".to_string()
        } else {
            preview
        }
    })
}

fn apply_custom_field_edit(
    draft: &mut CreateDraft,
    id: &str,
    text: &str,
) -> Result<String, String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        draft.extra_fields.remove(id);
        return Ok("(empty)".into());
    }
    if let Ok(n) = trimmed.parse::<f64>() {
        let val = serde_json::from_str(trimmed).unwrap_or_else(|_| serde_json::json!(n));
        draft.extra_fields.insert(id.to_string(), val);
        return Ok(trimmed.to_string());
    }
    if let Ok(d) = NaiveDate::parse_from_str(trimmed, "%Y-%m-%d") {
        let date_s = d.format("%Y-%m-%d").to_string();
        let val = serde_json::json!(date_s);
        draft.extra_fields.insert(id.to_string(), val.clone());
        return Ok(crate::api::types::format_custom_field_value(&val));
    }
    let val = serde_json::json!({ "value": trimmed });
    draft.extra_fields.insert(id.to_string(), val.clone());
    Ok(crate::api::types::format_custom_field_value(&val))
}

fn custom_field_edit_text(draft: &CreateDraft, id: &str) -> String {
    let formatted = crate::api::types::format_custom_field_value(
        draft
            .extra_fields
            .get(id)
            .unwrap_or(&serde_json::Value::Null),
    );
    if formatted == "-" {
        String::new()
    } else {
        formatted
    }
}

fn sprint_preview_text(draft: &CreateDraft, sprint_field: Option<&str>) -> String {
    let Some(sf) = sprint_field else {
        return String::new();
    };
    draft
        .extra_fields
        .get(sf)
        .and_then(|v| v.get("name").and_then(|n| n.as_str()))
        .map(String::from)
        .unwrap_or_default()
}

fn preview_line(s: &str, max_chars: usize) -> String {
    let one_line: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if one_line.is_empty() {
        return "(empty)".to_string();
    }
    if one_line.chars().count() <= max_chars {
        one_line
    } else {
        let mut end = 0;
        for (i, _) in one_line.char_indices().take(max_chars) {
            end = i;
        }
        format!("{}…", &one_line[..=end])
    }
}

pub fn touch_row_after_edit(row: &mut TemplateFieldRow, preview: String) {
    row.include = true;
    row.clear_value = false;
    row.preview = preview;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn sample_draft() -> CreateDraft {
        CreateDraft {
            summary: "Fix payment".into(),
            labels: vec!["bug".into(), "triage".into()],
            priority_name: "High".into(),
            due_date: NaiveDate::from_ymd_opt(2026, 6, 1),
            ..Default::default()
        }
    }

    #[test]
    fn editable_excludes_sprint() {
        assert!(!duplicate_field_is_editable(&TemplateFieldId::Sprint));
        assert!(duplicate_field_is_editable(&TemplateFieldId::Summary));
    }

    #[test]
    fn applies_label_and_due_date_edits() {
        let mut draft = sample_draft();
        let preview =
            apply_duplicate_row_edit(&mut draft, &TemplateFieldId::Labels, None, "a, b").unwrap();
        assert_eq!(preview, "a, b");
        assert_eq!(draft.labels, vec!["a", "b"]);

        let preview =
            apply_duplicate_row_edit(&mut draft, &TemplateFieldId::DueDate, None, "2026-07-04")
                .unwrap();
        assert_eq!(preview, "2026-07-04");
    }

    #[test]
    fn custom_field_number_and_clear() {
        let mut draft = CreateDraft {
            extra_fields: HashMap::from([("customfield_1".into(), serde_json::json!(3.5))]),
            ..Default::default()
        };
        let preview = apply_duplicate_row_edit(
            &mut draft,
            &TemplateFieldId::Custom("customfield_1".into()),
            None,
            "8",
        )
        .unwrap();
        assert_eq!(preview, "8");
        let preview = apply_duplicate_row_edit(
            &mut draft,
            &TemplateFieldId::Custom("customfield_1".into()),
            None,
            "",
        )
        .unwrap();
        assert_eq!(preview, "(empty)");
        assert!(!draft.extra_fields.contains_key("customfield_1"));
    }
}
