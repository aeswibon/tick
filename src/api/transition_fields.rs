//! Parse required transition fields and build Jira `fields` payloads.

use serde_json::{json, Value};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionField {
    pub id: String,
    pub name: String,
    pub field_type: String,
    /// `(id, label)` from Jira `allowedValues`, when present.
    pub options: Vec<(String, String)>,
}

impl TransitionField {
    pub fn value_from_choice(&self, id: &str, label: &str) -> Value {
        match self.field_type.as_str() {
            "resolution" => json!({ "name": label }),
            "user" => json!({ "accountId": id }),
            "priority" => json!({ "id": id }),
            _ if self.id == "resolution" => json!({ "name": label }),
            _ => json!({ "id": id }),
        }
    }

    pub fn value_from_text(&self, text: &str) -> Value {
        let trimmed = text.trim();
        match self.field_type.as_str() {
            "string" => json!(trimmed),
            "number" => trimmed
                .parse::<f64>()
                .map(|n| json!(n))
                .unwrap_or_else(|_| json!(trimmed)),
            _ => json!({ "value": trimmed }),
        }
    }
}

/// Fields on the transition screen that need user input before POST.
pub fn parse_transition_screen_fields(fields_obj: Option<&Value>) -> Vec<TransitionField> {
    let Some(obj) = fields_obj.and_then(|v| v.as_object()) else {
        return Vec::new();
    };
    let mut out: Vec<TransitionField> = obj
        .iter()
        .filter_map(|(id, meta)| {
            if !field_needs_input(meta) {
                return None;
            }
            let name = meta
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or(id)
                .to_string();
            let field_type = meta
                .get("schema")
                .and_then(|s| s.get("type"))
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string();
            let options = meta
                .get("allowedValues")
                .and_then(|a| a.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(parse_allowed_value)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            Some(TransitionField {
                id: id.clone(),
                name,
                field_type,
                options,
            })
        })
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

fn field_needs_input(meta: &Value) -> bool {
    if meta.get("required").and_then(|r| r.as_bool()) == Some(true) {
        return true;
    }
    if meta
        .get("allowedValues")
        .and_then(|a| a.as_array())
        .is_some_and(|a| !a.is_empty())
    {
        return true;
    }
    let field_type = meta
        .get("schema")
        .and_then(|s| s.get("type"))
        .and_then(|t| t.as_str())
        .unwrap_or("");
    let system = meta
        .get("schema")
        .and_then(|s| s.get("system"))
        .and_then(|t| t.as_str())
        .unwrap_or("");
    matches!(
        field_type,
        "resolution" | "priority" | "user" | "option" | "array" | "version" | "component"
    ) || matches!(
        system,
        "resolution" | "priority" | "assignee" | "fixVersions" | "components"
    )
}

/// When Jira omits `fields` on the list response, still prompt for resolution on Done/Close transitions.
pub fn infer_resolution_if_done_transition(
    transition_name: &str,
    to_status: &str,
) -> Option<TransitionField> {
    let done_like = |s: &str| {
        let s = s.to_lowercase();
        s.contains("done")
            || s.contains("closed")
            || s.contains("close")
            || s.contains("complete")
            || s.contains("resolved")
    };
    if done_like(transition_name) || done_like(to_status) {
        Some(TransitionField {
            id: "resolution".into(),
            name: "Resolution".into(),
            field_type: "resolution".into(),
            options: Vec::new(),
        })
    } else {
        None
    }
}

fn parse_allowed_value(v: &Value) -> Option<(String, String)> {
    if let Some(id) = v.get("id") {
        let id_s = id
            .as_str()
            .map(String::from)
            .or_else(|| id.as_i64().map(|n| n.to_string()))?;
        let label = v
            .get("name")
            .or_else(|| v.get("value"))
            .and_then(|l| l.as_str())
            .unwrap_or(&id_s)
            .to_string();
        return Some((id_s, label));
    }
    if let Some(aid) = v.get("accountId").and_then(|a| a.as_str()) {
        let label = v
            .get("displayName")
            .and_then(|d| d.as_str())
            .unwrap_or(aid)
            .to_string();
        return Some((aid.to_string(), label));
    }
    None
}

pub fn field_for_error_key(key: &str, known: &[TransitionField]) -> TransitionField {
    known
        .iter()
        .find(|f| f.id == key)
        .cloned()
        .unwrap_or_else(|| TransitionField {
            id: key.to_string(),
            name: humanize_field_id(key),
            field_type: String::new(),
            options: Vec::new(),
        })
}

fn humanize_field_id(id: &str) -> String {
    match id {
        "resolution" => "Resolution".into(),
        "assignee" => "Assignee".into(),
        "fixVersions" => "Fix versions".into(),
        "components" => "Components".into(),
        other => {
            let mut s = other.replace("customfield_", "Custom field ");
            if let Some(c) = s.get_mut(0..1) {
                c.make_ascii_uppercase();
            }
            s
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_required_resolution_with_options() {
        let fields = json!({
            "resolution": {
                "required": true,
                "name": "Resolution",
                "schema": { "type": "resolution", "system": "resolution" },
                "allowedValues": [
                    { "id": "10000", "name": "Done" },
                    { "id": "10001", "name": "Won't Do" }
                ]
            },
            "summary": { "required": false, "name": "Summary" }
        });
        let req = parse_transition_screen_fields(Some(&fields));
        assert_eq!(req.len(), 1);
        assert_eq!(req[0].id, "resolution");
        assert_eq!(req[0].options.len(), 2);
        assert_eq!(
            req[0].value_from_choice("10000", "Done"),
            json!({ "name": "Done" })
        );
    }
}
