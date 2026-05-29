//! Create issue, project search, create metadata, clone fetch, issue links.

use super::jira_error;
use super::transition_fields::{self, TransitionField};
use super::JiraClient;
use crate::api::types::Ticket;
use crate::config::{IssueTemplate, Site};
use chrono::NaiveDate;
use serde_json::{json, Value};

/// Failed create POST; `field_errors` lists Jira `errors` keys when present.
#[derive(Debug, Clone)]
pub struct CreateError {
    pub message: String,
    pub field_errors: Vec<(String, String)>,
}

/// Draft fields accumulated through the create/duplicate wizard.
#[derive(Debug, Clone, Default)]
pub struct CreateDraft {
    pub site_name: String,
    pub base_url: String,
    pub project_key: String,
    pub issue_type_name: String,
    pub summary: String,
    pub description: String,
    pub description_adf: Option<Value>,
    pub labels: Vec<String>,
    pub priority_id: Option<String>,
    pub priority_name: String,
    pub assignee_account_id: Option<String>,
    pub due_date: Option<NaiveDate>,
    pub parent_key: Option<String>,
    pub extra_fields: std::collections::HashMap<String, Value>,
    pub source_key: Option<String>,
}

impl JiraClient {
    pub async fn search_projects(&self, base_url: &str) -> Result<Vec<(String, String)>, String> {
        let base = base_url.trim_end_matches('/');
        let url = format!("{base}/rest/api/3/project/search");
        let resp = self
            .send(|| {
                self.get(&url)
                    .query(&[("maxResults", "50"), ("orderBy", "key")])
                    .send()
            })
            .await?;
        if !resp.status().is_success() {
            return Err(format!(
                "Project search {}: {}",
                resp.status(),
                resp.text().await.unwrap_or_default()
            ));
        }
        let data: Value = resp.json().await.map_err(|e| format!("Parse error: {e}"))?;
        let mut out: Vec<(String, String)> = data
            .get("values")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|p| {
                        let key = p.get("key")?.as_str()?;
                        let name = p.get("name")?.as_str()?;
                        Some((key.to_string(), format!("{key} — {name}")))
                    })
                    .collect()
            })
            .unwrap_or_default();
        out.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(out)
    }

    pub async fn list_issue_types_for_project(
        &self,
        base_url: &str,
        project_key: &str,
    ) -> Result<Vec<(String, String)>, String> {
        let base = base_url.trim_end_matches('/');
        let url = format!("{base}/rest/api/3/issue/createmeta");
        let resp = self
            .send(|| {
                self.get(&url)
                    .query(&[
                        ("projectKeys", project_key),
                        ("expand", "projects.issuetypes"),
                    ])
                    .send()
            })
            .await?;
        if !resp.status().is_success() {
            return Err(format!(
                "Create metadata {}: {}",
                resp.status(),
                resp.text().await.unwrap_or_default()
            ));
        }
        let data: Value = resp.json().await.map_err(|e| format!("Parse error: {e}"))?;
        let mut out = Vec::new();
        if let Some(projects) = data.get("projects").and_then(|p| p.as_array()) {
            for project in projects {
                if let Some(types) = project.get("issuetypes").and_then(|t| t.as_array()) {
                    for it in types {
                        let name = it
                            .get("name")
                            .and_then(|n| n.as_str())
                            .unwrap_or("")
                            .to_string();
                        if !name.is_empty() && !out.iter().any(|(_, n)| n == &name) {
                            out.push((name.clone(), name));
                        }
                    }
                }
            }
        }
        out.sort_by(|a, b| a.1.cmp(&b.1));
        Ok(out)
    }

    pub async fn required_fields_for_create(
        &self,
        base_url: &str,
        project_key: &str,
        issue_type_name: &str,
    ) -> Result<Vec<TransitionField>, String> {
        let fields_obj = self
            .fetch_create_meta_fields(base_url, project_key, Some(issue_type_name))
            .await?;
        Ok(transition_fields::parse_create_fields(&fields_obj))
    }

    async fn fetch_create_meta_fields(
        &self,
        base_url: &str,
        project_key: &str,
        issue_type_name: Option<&str>,
    ) -> Result<Value, String> {
        let base = base_url.trim_end_matches('/');
        let url = format!("{base}/rest/api/3/issue/createmeta");
        let req_builder = |req: reqwest::RequestBuilder| {
            let mut req = req.query(&[
                ("projectKeys", project_key),
                ("expand", "projects.issuetypes.fields"),
            ]);
            if let Some(name) = issue_type_name {
                req = req.query(&[("issuetypeNames", name)]);
            }
            req
        };
        let resp = self.send(|| req_builder(self.get(&url)).send()).await?;
        if !resp.status().is_success() {
            return Err(format!(
                "Create metadata {}: {}",
                resp.status(),
                resp.text().await.unwrap_or_default()
            ));
        }
        let data: Value = resp.json().await.map_err(|e| format!("Parse error: {e}"))?;
        let fields = data
            .get("projects")
            .and_then(|p| p.as_array())
            .and_then(|projects| projects.first())
            .and_then(|proj| proj.get("issuetypes"))
            .and_then(|types| types.as_array())
            .and_then(|types| {
                if let Some(name) = issue_type_name {
                    types
                        .iter()
                        .find(|t| {
                            t.get("name")
                                .and_then(|n| n.as_str())
                                .is_some_and(|n| n.eq_ignore_ascii_case(name))
                        })
                        .or_else(|| types.first())
                } else {
                    types.first()
                }
            })
            .and_then(|it| it.get("fields"))
            .cloned()
            .unwrap_or(json!({}));
        Ok(fields)
    }

    pub async fn fetch_issue_for_clone(
        &self,
        base_url: &str,
        key: &str,
        sprint_field: Option<&str>,
    ) -> Result<Value, String> {
        let mut field_list = vec![
            "summary",
            "description",
            "issuetype",
            "project",
            "labels",
            "priority",
            "assignee",
            "duedate",
            "parent",
        ];
        if let Some(sf) = sprint_field {
            field_list.push(sf);
        }
        let fields = field_list.join(",");
        let url = format!(
            "{}/rest/api/3/issue/{key}?fields={fields}",
            base_url.trim_end_matches('/')
        );
        let resp = self.send(|| self.get(&url).send()).await?;
        if !resp.status().is_success() {
            return Err(format!(
                "Issue fetch {}: {}",
                resp.status(),
                resp.text().await.unwrap_or_default()
            ));
        }
        resp.json().await.map_err(|e| format!("Parse error: {e}"))
    }

    pub async fn create_issue(
        &self,
        base_url: &str,
        fields: &Value,
    ) -> Result<String, CreateError> {
        let url = format!("{}/rest/api/3/issue", base_url.trim_end_matches('/'));
        let payload = json!({ "fields": fields });
        let resp = self
            .send(|| self.post(&url).json(&payload).send())
            .await
            .map_err(|e| CreateError {
                message: e,
                field_errors: Vec::new(),
            })?;
        let status = resp.status();
        if status.is_success() {
            let data: Value = resp.json().await.map_err(|e| CreateError {
                message: format!("Parse error: {e}"),
                field_errors: Vec::new(),
            })?;
            return data
                .get("key")
                .and_then(|k| k.as_str())
                .map(|s| s.to_string())
                .ok_or_else(|| CreateError {
                    message: "Create succeeded but no issue key in response".into(),
                    field_errors: Vec::new(),
                });
        }
        let body = resp.text().await.unwrap_or_default();
        let detail = jira_error::format_response_error(status, &body);
        Err(CreateError {
            message: format!("Cannot create issue: {detail}"),
            field_errors: jira_error::field_errors(&body),
        })
    }

    pub async fn link_issues_clones(
        &self,
        base_url: &str,
        inward_key: &str,
        outward_key: &str,
        link_type_name: &str,
    ) -> Result<(), String> {
        let url = format!("{}/rest/api/3/issueLink", base_url.trim_end_matches('/'));
        let payload = json!({
            "type": { "name": link_type_name },
            "inwardIssue": { "key": inward_key },
            "outwardIssue": { "key": outward_key },
        });
        let resp = self.send(|| self.post(&url).json(&payload).send()).await?;
        if resp.status().is_success() {
            Ok(())
        } else {
            Err(format!(
                "Issue link {}: {}",
                resp.status(),
                resp.text().await.unwrap_or_default()
            ))
        }
    }

    pub async fn resolve_priority_id(&self, base_url: &str, priority_name: &str) -> Option<String> {
        if priority_name.is_empty() {
            return None;
        }
        let list = self.list_priorities(base_url).await.ok()?;
        list.into_iter()
            .find(|(_, name)| name.eq_ignore_ascii_case(priority_name))
            .map(|(id, _)| id)
    }
}

/// Build Jira `fields` object for create from draft + collected required custom fields.
pub fn build_create_fields(
    draft: &CreateDraft,
    required_values: &std::collections::HashMap<String, Value>,
) -> Value {
    let mut fields = json!({
        "project": { "key": draft.project_key },
        "issuetype": { "name": draft.issue_type_name },
        "summary": draft.summary,
    });

    if let Some(adf) = &draft.description_adf {
        fields["description"] = adf.clone();
    } else if !draft.description.trim().is_empty() {
        fields["description"] = super::markdown::to_adf(&draft.description, &[]);
    }

    if !draft.labels.is_empty() {
        fields["labels"] = json!(draft.labels);
    }
    if let Some(id) = &draft.priority_id {
        fields["priority"] = json!({ "id": id });
    }
    if let Some(account_id) = &draft.assignee_account_id {
        fields["assignee"] = json!({ "accountId": account_id });
    }
    if let Some(d) = draft.due_date {
        fields["duedate"] = json!(d.format("%Y-%m-%d").to_string());
    }
    if let Some(parent) = &draft.parent_key {
        fields["parent"] = json!({ "key": parent });
    }

    if let Some(obj) = fields.as_object_mut() {
        for (k, v) in required_values {
            obj.insert(k.clone(), v.clone());
        }
        for (k, v) in &draft.extra_fields {
            obj.insert(k.clone(), v.clone());
        }
        // Don't send summary/description twice from required_values if duplicated
        obj.remove("summary");
        obj.insert("summary".into(), json!(draft.summary));
    }

    fields
}

/// Apply clone GET response onto a draft (maximal copy).
pub async fn enrich_draft_from_clone(
    jira: &JiraClient,
    draft: &mut CreateDraft,
    issue: &Value,
    sprint_field: Option<&str>,
) {
    let fields = issue.get("fields").unwrap_or(issue);
    if let Some(s) = fields.get("summary").and_then(|v| v.as_str()) {
        draft.summary = s.to_string();
    }
    if let Some(desc) = fields.get("description") {
        if !desc.is_null() {
            draft.description_adf = Some(desc.clone());
            draft.description = super::adf_export::to_markdown(desc);
        }
    }
    if let Some(labels) = fields.get("labels").and_then(|l| l.as_array()) {
        draft.labels = labels
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();
    }
    if let Some(p) = fields.get("priority") {
        draft.priority_name = p
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("")
            .to_string();
        draft.priority_id = p.get("id").and_then(|id| id.as_str()).map(String::from);
    }
    if let Some(a) = fields.get("assignee") {
        draft.assignee_account_id = a
            .get("accountId")
            .and_then(|id| id.as_str())
            .map(String::from);
    }
    if let Some(d) = fields.get("duedate").and_then(|v| v.as_str()) {
        draft.due_date = NaiveDate::parse_from_str(d, "%Y-%m-%d").ok();
    }
    if let Some(parent) = fields.get("parent") {
        draft.parent_key = parent.get("key").and_then(|k| k.as_str()).map(String::from);
    }
    if let Some(it) = fields.get("issuetype") {
        draft.issue_type_name = it
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or(&draft.issue_type_name)
            .to_string();
    }
    if let Some(proj) = fields.get("project") {
        draft.project_key = proj
            .get("key")
            .and_then(|k| k.as_str())
            .unwrap_or(&draft.project_key)
            .to_string();
    }
    if draft.priority_id.is_none() && !draft.priority_name.is_empty() {
        draft.priority_id = jira
            .resolve_priority_id(&draft.base_url, &draft.priority_name)
            .await;
    }
    if let Some(sf) = sprint_field {
        if let Some(val) = fields.get(sf) {
            if !val.is_null() {
                draft.extra_fields.insert(sf.to_string(), val.clone());
            }
        }
    }
}

/// Apply a configured template; caller resolves `priority_id` if `priority` is set.
pub fn apply_template_to_draft(draft: &mut CreateDraft, template: &IssueTemplate, site: &Site) {
    draft.site_name = site.name.clone();
    draft.base_url = site.base_url.clone();
    draft.project_key = template.project.trim().to_string();
    draft.issue_type_name = template.issue_type.trim().to_string();
    draft.summary = template.summary.clone();
    draft.description = template.description.clone();
    draft.description_adf = None;
    draft.labels = template.labels.clone();
    draft.priority_name = template.priority.clone().unwrap_or_default();
    draft.priority_id = None;
    draft.assignee_account_id = template.assignee_account_id.clone();
    draft.parent_key = template.parent_key.clone();
    draft.source_key = None;
    draft.extra_fields.clear();
    for (key, value) in &template.extra_fields {
        if let Ok(json) = serde_json::to_value(value) {
            draft.extra_fields.insert(key.clone(), json);
        }
    }
}

pub fn template_picker_label(template: &IssueTemplate) -> String {
    let site = template
        .site
        .as_deref()
        .map(|s| format!("{s} · "))
        .unwrap_or_default();
    format!(
        "{}{} — {} / {}",
        site, template.name, template.project, template.issue_type
    )
}

pub fn seed_draft_from_ticket(draft: &mut CreateDraft, ticket: &Ticket, summary_prefix: &str) {
    draft.site_name = ticket.site.clone();
    draft.project_key = ticket.project_key_for_api().to_string();
    draft.issue_type_name = ticket.issue_type.clone();
    draft.summary = format!("{summary_prefix}{}", ticket.summary);
    draft.labels = ticket.labels.clone();
    draft.priority_name = ticket.priority.clone();
    draft.due_date = ticket.due_date;
    draft.parent_key = ticket.parent_key.clone();
    draft.source_key = Some(ticket.key.clone());
    if let Some(adf) = &ticket.description_adf {
        draft.description_adf = Some(adf.clone());
        draft.description = super::adf_export::to_markdown(adf);
    } else if let Some(ref text) = ticket.description {
        draft.description = text.clone();
        draft.description_adf = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn build_create_fields_includes_core() {
        let draft = CreateDraft {
            project_key: "DEMO".into(),
            issue_type_name: "Task".into(),
            summary: "Test issue".into(),
            priority_id: Some("3".into()),
            labels: vec!["a".into()],
            ..Default::default()
        };
        let fields = build_create_fields(&draft, &std::collections::HashMap::new());
        assert_eq!(fields["project"]["key"], "DEMO");
        assert_eq!(fields["summary"], "Test issue");
        assert_eq!(fields["labels"][0], "a");
    }

    #[test]
    fn apply_template_fills_draft() {
        use crate::config::IssueTemplate;
        let template = IssueTemplate {
            name: "inc".into(),
            site: Some("s1".into()),
            project: "OPS".into(),
            issue_type: "Task".into(),
            summary: "Incident: ".into(),
            description: "details".into(),
            labels: vec!["ops".into()],
            priority: Some("High".into()),
            ..Default::default()
        };
        let site = Site {
            name: "s1".into(),
            base_url: "https://x.atlassian.net".into(),
            ..Default::default()
        };
        let mut draft = CreateDraft::default();
        apply_template_to_draft(&mut draft, &template, &site);
        assert_eq!(draft.project_key, "OPS");
        assert_eq!(draft.summary, "Incident: ");
        assert_eq!(draft.labels, vec!["ops"]);
    }

    #[test]
    fn seed_draft_prefixes_summary() {
        let ticket = Ticket {
            key: "DEMO-1".into(),
            site: "s".into(),
            issue_type: "Bug".into(),
            status: String::new(),
            status_color: String::new(),
            priority: String::new(),
            ageing_days: 0,
            due_date: None,
            assignee: String::new(),
            reporter: String::new(),
            summary: "Fix it".into(),
            link: String::new(),
            description: None,
            description_adf: None,
            latest_comment: None,
            all_comments: Vec::new(),
            parent_key: None,
            parent_summary: None,
            labels: Vec::new(),
            sprint_name: None,
            project_key: "DEMO".into(),
        };
        let mut draft = CreateDraft::default();
        seed_draft_from_ticket(&mut draft, &ticket, "Copy of: ");
        assert_eq!(draft.summary, "Copy of: Fix it");
        assert_eq!(draft.issue_type_name, "Bug");
    }
}
