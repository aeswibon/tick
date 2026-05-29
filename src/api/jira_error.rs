//! Parse Jira REST error bodies into user-facing messages.

use reqwest::StatusCode;
use serde_json::Value;

pub fn format_response_error(status: StatusCode, body: &str) -> String {
    let context = match status {
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
            "Check API token, email, and project permissions"
        }
        StatusCode::NOT_FOUND => "Issue or resource not found",
        StatusCode::BAD_REQUEST => "Request rejected by Jira",
        StatusCode::UNPROCESSABLE_ENTITY => "Workflow or field validation failed",
        _ => "Jira API error",
    };
    if let Some(detail) = parse_jira_body(body) {
        format!("{context}: {detail}")
    } else {
        format!("{context} (HTTP {})", status.as_u16())
    }
}

/// Field keys from Jira's `errors` object and common `errorMessages` text.
pub fn field_errors(body: &str) -> Vec<(String, String)> {
    let Ok(v) = serde_json::from_str::<Value>(body) else {
        return Vec::new();
    };
    let mut out: Vec<(String, String)> = v
        .get("errors")
        .and_then(|e| e.as_object())
        .map(|obj| {
            obj.iter()
                .map(|(field, msg)| {
                    let text = msg.as_str().unwrap_or("required").to_string();
                    (field.clone(), text)
                })
                .collect()
        })
        .unwrap_or_default();

    if let Some(msgs) = v.get("errorMessages").and_then(|m| m.as_array()) {
        for m in msgs {
            if let Some(s) = m.as_str() {
                if let Some(id) = infer_field_from_message(s) {
                    if !out.iter().any(|(k, _)| k == &id) {
                        out.push((id, s.to_string()));
                    }
                }
            }
        }
    }
    out
}

fn infer_field_from_message(msg: &str) -> Option<String> {
    let lower = msg.to_lowercase();
    if lower.contains("resolution") {
        return Some("resolution".into());
    }
    if lower.contains("assignee") {
        return Some("assignee".into());
    }
    if lower.contains("fix version") || lower.contains("fixversions") {
        return Some("fixVersions".into());
    }
    if lower.contains("component") {
        return Some("components".into());
    }
    if lower.contains("priority") {
        return Some("priority".into());
    }
    None
}

fn parse_jira_body(body: &str) -> Option<String> {
    let v: Value = serde_json::from_str(body).ok()?;
    let mut parts = Vec::new();
    if let Some(msgs) = v.get("errorMessages").and_then(|m| m.as_array()) {
        for m in msgs {
            if let Some(s) = m.as_str() {
                if !s.is_empty() {
                    parts.push(s.to_string());
                }
            }
        }
    }
    if let Some(errors) = v.get("errors").and_then(|e| e.as_object()) {
        for (field, msg) in errors {
            let text = msg.as_str().unwrap_or("");
            if text.is_empty() {
                parts.push(field.clone());
            } else {
                parts.push(format!("{field}: {text}"));
            }
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("; "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_errors_from_error_messages() {
        let body = r#"{"errorMessages":["Resolution is required"],"errors":{}}"#;
        let errs = field_errors(body);
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].0, "resolution");
    }

    #[test]
    fn field_errors_extracts_keys() {
        let body = r#"{"errors":{"resolution":"Resolution is required"}}"#;
        let errs = field_errors(body);
        assert_eq!(
            errs,
            vec![("resolution".into(), "Resolution is required".into())]
        );
    }

    #[test]
    fn parses_error_messages_and_errors() {
        let body =
            r#"{"errorMessages":["Resolution is required"],"errors":{"assignee":"required"}}"#;
        let msg = format_response_error(StatusCode::BAD_REQUEST, body);
        assert!(msg.contains("Resolution is required"));
        assert!(msg.contains("assignee"));
    }
}
