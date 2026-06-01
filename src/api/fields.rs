//! Jira field catalog and per-issue edit metadata (custom field phase 2).

use serde::Serialize;
use serde_json::{json, Value};

use super::transition_fields::{self, TransitionField, TransitionFieldKind};
use super::JiraClient;

/// Entry from `GET /rest/api/3/field` (for CLI / doctor).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FieldCatalogEntry {
    pub id: String,
    pub name: String,
    pub custom: bool,
    pub schema_type: String,
    pub schema_custom: String,
    pub system: String,
    /// Suggested `[[detail.editable_fields]]` type: `text`, `select`, `user`, `auto`, or `unsupported`.
    pub suggested_type: String,
}

impl JiraClient {
    /// List fields from Jira (`GET /rest/api/3/field`).
    pub async fn list_field_catalog(
        &self,
        base_url: &str,
        custom_only: bool,
    ) -> Result<Vec<FieldCatalogEntry>, String> {
        let url = format!("{}/rest/api/3/field", base_url.trim_end_matches('/'));
        let resp = self.send(|| self.get(&url).send()).await?;
        if !resp.status().is_success() {
            return Err(format!(
                "Field API {}: {}",
                resp.status(),
                resp.text().await.unwrap_or_default()
            ));
        }
        let data: Value = resp.json().await.map_err(|e| format!("Parse error: {e}"))?;
        let mut out: Vec<FieldCatalogEntry> = data
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|f| parse_catalog_entry(f, custom_only))
                    .collect()
            })
            .unwrap_or_default();
        out.sort_by_key(|a| a.name.to_lowercase());
        Ok(out)
    }

    /// Create-screen field metadata for a project (first issue type) — includes `allowedValues`.
    pub async fn list_createmeta_fields_for_project(
        &self,
        base_url: &str,
        project_key: &str,
    ) -> Result<Value, String> {
        self.fetch_create_meta_fields(base_url, project_key, None)
            .await
    }

    /// Field metadata for editing an existing issue (`GET .../editmeta`).
    pub async fn fetch_editmeta_field(
        &self,
        base_url: &str,
        key: &str,
        field_id: &str,
    ) -> Result<Option<TransitionField>, String> {
        let base = base_url.trim_end_matches('/');
        let url = format!("{base}/rest/api/3/issue/{key}/editmeta");
        let resp = self.send(|| self.get(&url).send()).await?;
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(format!("Issue {key} not found"));
        }
        if !resp.status().is_success() {
            return Err(format!(
                "Edit metadata {}: {}",
                resp.status(),
                resp.text().await.unwrap_or_default()
            ));
        }
        let data: Value = resp.json().await.map_err(|e| format!("Parse error: {e}"))?;
        let Some(meta) = data.get("fields").and_then(|f| f.get(field_id)) else {
            return Ok(None);
        };
        let wrapped = json!({ field_id: meta });
        Ok(
            transition_fields::parse_transition_screen_fields(Some(&wrapped))
                .into_iter()
                .next(),
        )
    }
}

fn parse_catalog_entry(f: &Value, custom_only: bool) -> Option<FieldCatalogEntry> {
    let id = f.get("id")?.as_str()?;
    let custom = f.get("custom").and_then(|c| c.as_bool()).unwrap_or(false);
    if custom_only && !custom {
        return None;
    }
    if custom_only && !id.starts_with("customfield_") {
        return None;
    }
    let name = f.get("name")?.as_str()?.to_string();
    let schema = f.get("schema");
    let schema_type = schema
        .and_then(|s| s.get("type"))
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .to_string();
    let schema_custom = schema
        .and_then(|s| s.get("custom"))
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .to_string();
    let system = schema
        .and_then(|s| s.get("system"))
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .to_string();
    let suggested_type = suggest_tick_type(&schema_type, &system, &schema_custom, custom);
    Some(FieldCatalogEntry {
        id: id.to_string(),
        name,
        custom,
        schema_type,
        schema_custom,
        system,
        suggested_type,
    })
}

pub fn suggest_tick_type(
    schema_type: &str,
    system: &str,
    schema_custom: &str,
    custom: bool,
) -> String {
    if !custom && !system.is_empty() {
        return "unsupported".into();
    }
    match schema_type {
        "user" => "user".into(),
        "option" => "select".into(),
        "array" if schema_custom.contains("multiselect") || schema_custom.contains("checkbox") => {
            "unsupported".into()
        }
        "string"
            if schema_custom.contains("select")
                || schema_custom.contains("radiobuttons")
                || schema_custom.contains("multicheckboxes") =>
        {
            "select".into()
        }
        "number" | "string" => "text".into(),
        "date" | "datetime" => "auto".into(),
        _ => {
            if schema_custom.contains(":userpicker") {
                "user".into()
            } else if schema_custom.contains("select") {
                "select".into()
            } else {
                "auto".into()
            }
        }
    }
}

/// Map editmeta to config `type` string.
pub fn tick_type_from_transition_field(tf: &TransitionField) -> &'static str {
    match tf.kind {
        TransitionFieldKind::User => "user",
        TransitionFieldKind::Picker | TransitionFieldKind::Boolean => "select",
        TransitionFieldKind::Text | TransitionFieldKind::Number => "text",
        TransitionFieldKind::Date | TransitionFieldKind::DateTime => "text",
        TransitionFieldKind::MultiPicker => "unsupported",
    }
}

pub fn select_options_from_transition_field(tf: &TransitionField) -> Vec<String> {
    tf.options.iter().map(|(_, label)| label.clone()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn list_field_catalog_custom_only() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/rest/api/3/field"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                {
                    "id": "summary",
                    "name": "Summary",
                    "custom": false,
                    "schema": { "type": "string", "system": "summary" }
                },
                {
                    "id": "customfield_10042",
                    "name": "Story Points",
                    "custom": true,
                    "schema": {
                        "type": "number",
                        "custom": "com.atlassian.jira.plugin.system.customfieldtypes:float"
                    }
                }
            ])))
            .mount(&server)
            .await;

        let client = crate::api::JiraClient::new("a@b.com", "t", false);
        let list = client
            .list_field_catalog(&server.uri(), true)
            .await
            .unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "customfield_10042");
        assert_eq!(list[0].suggested_type, "text");
    }

    #[test]
    fn suggest_types_for_common_custom_fields() {
        assert_eq!(
            suggest_tick_type(
                "number",
                "",
                "com.atlassian.jira.plugin.system.customfieldtypes:float",
                true
            ),
            "text"
        );
        assert_eq!(
            suggest_tick_type(
                "option",
                "",
                "com.atlassian.jira.plugin.system.customfieldtypes:select",
                true
            ),
            "select"
        );
        assert_eq!(
            suggest_tick_type(
                "user",
                "",
                "com.atlassian.jira.plugin.system.customfieldtypes:userpicker",
                true
            ),
            "user"
        );
    }
}
