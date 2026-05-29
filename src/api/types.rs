use chrono::NaiveDate;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CachedView {
    pub fetched_at: String,
    pub tickets: Vec<Ticket>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Ticket {
    pub key: String,
    pub site: String,
    pub issue_type: String,
    pub status: String,
    pub status_color: String,
    pub priority: String,
    pub ageing_days: i64,
    pub due_date: Option<NaiveDate>,
    pub assignee: String,
    pub reporter: String,
    pub summary: String,
    pub link: String,
    pub description: Option<String>,
    pub description_adf: Option<serde_json::Value>,
    pub latest_comment: Option<String>,
    pub all_comments: Vec<CommentEntry>,
    pub parent_key: Option<String>,
    pub parent_summary: Option<String>,
    pub labels: Vec<String>,
    pub sprint_name: Option<String>,
    #[serde(default)]
    pub project_key: String,
}

pub(crate) fn project_key_from_issue_key(key: &str) -> &str {
    key.rfind('-').map_or(key, |i| &key[..i])
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CommentEntry {
    pub author: String,
    pub created: String,
    pub body: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct JqlSearchResponse {
    pub issues: Vec<JqlIssue>,
    pub next_page_token: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct JqlIssue {
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct BulkFetchResponse {
    pub issues: Vec<BulkFetchIssue>,
}

#[derive(Debug, Deserialize)]
pub struct BulkFetchIssue {
    pub key: String,
    pub fields: JiraFields,
}

#[derive(Debug, Deserialize)]
pub struct JiraFields {
    #[serde(rename = "issuetype")]
    pub issue_type: JiraNamed,
    pub status: JiraStatus,
    pub priority: Option<JiraNamed>,
    pub assignee: Option<JiraUser>,
    pub reporter: Option<JiraUser>,
    pub duedate: Option<NaiveDate>,
    pub created: String,
    #[allow(dead_code)]
    pub project: JiraProject,
    pub summary: String,
    pub description: Option<serde_json::Value>,
    pub comment: Option<JiraComments>,
    pub parent: Option<JiraParent>,
    pub labels: Option<Vec<String>>,
    #[serde(flatten)]
    pub custom: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct JiraStatus {
    pub name: String,
    #[serde(rename = "statusCategory")]
    pub status_category: JiraStatusCategory,
}

#[derive(Debug, Deserialize)]
pub struct JiraStatusCategory {
    #[allow(dead_code)]
    pub key: String,
    #[serde(rename = "colorName")]
    pub color_name: String,
}

#[derive(Debug, Deserialize)]
pub struct JiraComments {
    pub comments: Vec<JiraComment>,
}

#[derive(Debug, Deserialize)]
pub struct JiraComment {
    pub body: Option<serde_json::Value>,
    #[allow(dead_code)]
    pub created: String,
    #[allow(dead_code)]
    pub author: Option<JiraUser>,
}

#[derive(Debug, Deserialize)]
pub struct JiraNamed {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct JiraUser {
    #[serde(rename = "displayName")]
    pub display_name: String,
}

#[derive(Debug, Deserialize)]
pub struct JiraProject {
    #[allow(dead_code)]
    pub key: String,
}

#[derive(Debug, Deserialize)]
pub struct JiraParent {
    pub key: String,
    pub fields: JiraParentFields,
}

#[derive(Debug, Deserialize)]
pub struct JiraParentFields {
    pub summary: String,
}

pub(crate) fn extract_text(v: &serde_json::Value) -> Option<String> {
    match v {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Object(m) => {
            let node_type = m.get("type").and_then(|t| t.as_str());
            if node_type == Some("mention") {
                return m
                    .get("attrs")
                    .and_then(|a| a.get("text"))
                    .and_then(|t| t.as_str())
                    .filter(|s| !s.is_empty())
                    .map(String::from);
            }
            if node_type == Some("hardBreak") {
                return Some("\n".into());
            }
            if let Some(content) = m.get("content").and_then(|c| c.as_array()) {
                let sep = match node_type {
                    Some("paragraph") => "",
                    _ => "\n",
                };
                let mut parts = Vec::new();
                for node in content {
                    if let Some(text) = extract_text(node) {
                        parts.push(text);
                    }
                }
                if parts.is_empty() {
                    None
                } else {
                    Some(parts.join(sep))
                }
            } else {
                m.get("text")
                    .and_then(|t| t.as_str())
                    .map(|text| text.to_string())
            }
        }
        _ => None,
    }
}

/// Collect `@mention` labels from an ADF document (description or comment body).
pub fn collect_mention_labels(doc: &serde_json::Value) -> Vec<String> {
    let mut labels = Vec::new();
    collect_mention_labels_rec(doc, &mut labels);
    labels.sort();
    labels.dedup();
    labels
}

fn collect_mention_labels_rec(node: &serde_json::Value, out: &mut Vec<String>) {
    if let Some(obj) = node.as_object() {
        if obj.get("type").and_then(|t| t.as_str()) == Some("mention") {
            if let Some(label) = obj
                .get("attrs")
                .and_then(|a| a.get("text"))
                .and_then(|t| t.as_str())
                .filter(|s| !s.is_empty())
            {
                out.push(label.to_string());
            }
        }
        if let Some(content) = obj.get("content").and_then(|c| c.as_array()) {
            for child in content {
                collect_mention_labels_rec(child, out);
            }
        }
    }
}

pub(crate) fn extract_sprint_name(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(s) if !s.is_empty() => Some(s.clone()),
        serde_json::Value::Object(m) => m
            .get("name")
            .and_then(|n| n.as_str())
            .filter(|s| !s.is_empty())
            .map(String::from),
        serde_json::Value::Array(arr) => arr.iter().find_map(extract_sprint_name),
        _ => None,
    }
}

impl Ticket {
    pub fn project_key_for_api(&self) -> &str {
        if !self.project_key.is_empty() {
            &self.project_key
        } else {
            project_key_from_issue_key(&self.key)
        }
    }

    pub fn from_bulk_fetch(
        issue: BulkFetchIssue,
        site_name: &str,
        base_url: &str,
        sprint_field: Option<&str>,
    ) -> Self {
        let ageing_days = NaiveDate::parse_from_str(&issue.fields.created[..10], "%Y-%m-%d")
            .map(|d| (chrono::Utc::now().date_naive() - d).num_days())
            .unwrap_or(0);

        let description_adf = issue.fields.description.clone();
        let description = description_adf.as_ref().and_then(extract_text);

        let all_comments: Vec<CommentEntry> = issue
            .fields
            .comment
            .as_ref()
            .map(|c| {
                c.comments
                    .iter()
                    .map(|cmt| CommentEntry {
                        author: cmt
                            .author
                            .as_ref()
                            .map(|a| a.display_name.clone())
                            .unwrap_or_default(),
                        created: cmt.created.clone(),
                        body: cmt.body.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default();

        let latest_comment = all_comments
            .last()
            .and_then(|c| c.body.as_ref())
            .and_then(extract_text);

        Self {
            key: issue.key.clone(),
            site: site_name.to_string(),
            issue_type: issue.fields.issue_type.name,
            status: issue.fields.status.name,
            status_color: issue.fields.status.status_category.color_name,
            priority: issue
                .fields
                .priority
                .as_ref()
                .map(|p| p.name.clone())
                .unwrap_or_default(),
            ageing_days,
            due_date: issue.fields.duedate,
            assignee: issue
                .fields
                .assignee
                .as_ref()
                .map(|u| u.display_name.clone())
                .unwrap_or_default(),
            reporter: issue
                .fields
                .reporter
                .as_ref()
                .map(|u| u.display_name.clone())
                .unwrap_or_default(),
            summary: issue.fields.summary,
            link: format!("{}/browse/{}", base_url.trim_end_matches('/'), issue.key),
            description,
            description_adf,
            latest_comment,
            all_comments,
            parent_key: issue.fields.parent.as_ref().map(|p| p.key.clone()),
            parent_summary: issue
                .fields
                .parent
                .as_ref()
                .map(|p| p.fields.summary.clone()),
            labels: issue.fields.labels.clone().unwrap_or_default(),
            sprint_name: sprint_field
                .and_then(|field| issue.fields.custom.get(field).and_then(extract_sprint_name)),
            project_key: issue.fields.project.key.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn project_key_from_issue_key_parses_prefix() {
        assert_eq!(project_key_from_issue_key("DEMO-1"), "DEMO");
        assert_eq!(project_key_from_issue_key("MY-PROJ-42"), "MY-PROJ");
    }

    #[test]
    fn extract_text_from_plain_string() {
        let v = serde_json::json!("hello");
        assert_eq!(extract_text(&v), Some("hello".into()));
    }

    #[test]
    fn extract_text_from_adf_paragraph() {
        let v = serde_json::json!({
            "type": "doc",
            "content": [{"type": "paragraph", "content": [{"type": "text", "text": "line one"}]}]
        });
        assert_eq!(extract_text(&v), Some("line one".into()));
    }

    #[test]
    fn collect_mention_labels_dedupes() {
        let doc = serde_json::json!({
            "type": "doc",
            "content": [{
                "type": "paragraph",
                "content": [
                    {"type": "mention", "attrs": {"text": "@Ada"}},
                    {"type": "mention", "attrs": {"text": "@Ada"}},
                    {"type": "mention", "attrs": {"text": "@Bob"}},
                ]
            }]
        });
        assert_eq!(
            collect_mention_labels(&doc),
            vec!["@Ada".to_string(), "@Bob".to_string()]
        );
    }

    #[test]
    fn extract_text_includes_mention_label() {
        let v = serde_json::json!({
            "type": "paragraph",
            "content": [
                {"type": "text", "text": "hi "},
                {"type": "mention", "attrs": {"id": "1", "text": "@Ada"}},
                {"type": "text", "text": "!"}
            ]
        });
        assert_eq!(extract_text(&v), Some("hi @Ada!".into()));
    }

    #[test]
    fn jql_search_response_parses_issue_ids() {
        let raw = r#"{"issues":[{"id":"10001"},{"id":"10002"}]}"#;
        let data: JqlSearchResponse = serde_json::from_str(raw).unwrap();
        assert_eq!(data.issues.len(), 2);
        assert_eq!(data.issues[0].id, "10001");
    }

    #[test]
    fn ticket_from_bulk_fetch_maps_fields() {
        let issue = BulkFetchIssue {
            key: "PROJ-1".into(),
            fields: JiraFields {
                issue_type: JiraNamed { name: "Bug".into() },
                status: JiraStatus {
                    name: "In Progress".into(),
                    status_category: JiraStatusCategory {
                        key: "indeterminate".into(),
                        color_name: "yellow".into(),
                    },
                },
                priority: Some(JiraNamed {
                    name: "High".into(),
                }),
                assignee: Some(JiraUser {
                    display_name: "Alice".into(),
                }),
                reporter: Some(JiraUser {
                    display_name: "Bob".into(),
                }),
                duedate: Some(NaiveDate::from_ymd_opt(2026, 6, 1).unwrap()),
                created: "2026-01-15T10:00:00.000+0000".into(),
                project: JiraProject { key: "PROJ".into() },
                summary: "Fix login".into(),
                description: Some(serde_json::json!("Plain description")),
                comment: None,
                parent: None,
                labels: Some(vec!["backend".into(), "urgent".into()]),
                custom: HashMap::new(),
            },
        };
        let ticket = Ticket::from_bulk_fetch(issue, "acme", "https://acme.atlassian.net", None);
        assert_eq!(ticket.key, "PROJ-1");
        assert_eq!(ticket.site, "acme");
        assert_eq!(ticket.status, "In Progress");
        assert_eq!(ticket.assignee, "Alice");
        assert_eq!(ticket.description.as_deref(), Some("Plain description"));
        assert!(ticket.link.contains("PROJ-1"));
        assert_eq!(ticket.labels, vec!["backend", "urgent"]);
    }

    #[test]
    fn extract_sprint_from_object_and_array() {
        let obj = serde_json::json!({ "name": "Sprint 1" });
        assert_eq!(extract_sprint_name(&obj), Some("Sprint 1".into()));
        let arr = serde_json::json!([{ "name": "Sprint 2" }]);
        assert_eq!(extract_sprint_name(&arr), Some("Sprint 2".into()));
    }

    #[test]
    fn ticket_maps_custom_sprint_field() {
        let mut custom = HashMap::new();
        custom.insert(
            "customfield_10020".into(),
            serde_json::json!({ "name": "Board Sprint" }),
        );
        let issue = BulkFetchIssue {
            key: "X-1".into(),
            fields: JiraFields {
                issue_type: JiraNamed {
                    name: "Task".into(),
                },
                status: JiraStatus {
                    name: "Open".into(),
                    status_category: JiraStatusCategory {
                        key: "new".into(),
                        color_name: "blue".into(),
                    },
                },
                priority: None,
                assignee: None,
                reporter: None,
                duedate: None,
                created: "2026-01-01T00:00:00.000+0000".into(),
                project: JiraProject { key: "X".into() },
                summary: "S".into(),
                description: None,
                comment: None,
                parent: None,
                labels: None,
                custom,
            },
        };
        let ticket = Ticket::from_bulk_fetch(
            issue,
            "acme",
            "https://acme.atlassian.net",
            Some("customfield_10020"),
        );
        assert_eq!(ticket.sprint_name.as_deref(), Some("Board Sprint"));
    }
}
