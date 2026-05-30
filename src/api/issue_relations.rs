//! Issue links and subtasks for the detail Links tab.

use super::JiraClient;
use serde::Deserialize;
use serde_json::json;

/// A linked issue shown relative to the current issue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssueLinkView {
    pub link_type: String,
    pub direction: String,
    pub other_key: String,
    pub other_summary: String,
    pub other_status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubtaskView {
    pub key: String,
    pub summary: String,
    pub status: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IssueRelations {
    pub links: Vec<IssueLinkView>,
    pub subtasks: Vec<SubtaskView>,
}

#[derive(Debug, Deserialize)]
struct IssueRelationsResponse {
    fields: IssueRelationsFields,
}

#[derive(Debug, Deserialize)]
struct IssueRelationsFields {
    #[serde(default)]
    issuelinks: Vec<JiraIssueLink>,
    #[serde(default)]
    subtasks: Vec<JiraSubtaskRef>,
}

#[derive(Debug, Deserialize)]
struct JiraIssueLink {
    #[serde(rename = "type")]
    link_type: JiraLinkType,
    #[serde(rename = "inwardIssue")]
    inward_issue: Option<JiraLinkedIssue>,
    #[serde(rename = "outwardIssue")]
    outward_issue: Option<JiraLinkedIssue>,
}

#[derive(Debug, Deserialize)]
struct JiraLinkType {
    name: String,
    inward: String,
    outward: String,
}

#[derive(Debug, Deserialize)]
struct JiraLinkedIssue {
    key: String,
    fields: JiraLinkedFields,
}

#[derive(Debug, Deserialize)]
struct JiraLinkedFields {
    summary: String,
    status: JiraLinkedStatus,
}

#[derive(Debug, Deserialize)]
struct JiraLinkedStatus {
    name: String,
}

#[derive(Debug, Deserialize)]
struct JiraSubtaskRef {
    key: String,
    fields: JiraSubtaskFields,
}

#[derive(Debug, Deserialize)]
struct JiraSubtaskFields {
    summary: String,
    status: JiraLinkedStatus,
}

/// Preset link types for the add-link picker: `(Jira type name, label)`.
pub const ADD_LINK_TYPES: &[(&str, &str)] = &[
    ("Relates", "Relates to"),
    ("Blocks", "Blocks"),
    ("Blocks", "Is blocked by"),
    ("Epic-Story Link", "Epic"),
];

/// Which picker index uses “blocked by” semantics (same Jira type as Blocks).
pub fn add_link_blocked_by(index: usize) -> bool {
    index == 2
}

impl JiraClient {
    pub async fn fetch_issue_relations(
        &self,
        base_url: &str,
        issue_key: &str,
    ) -> Result<IssueRelations, String> {
        let url = format!(
            "{}/rest/api/3/issue/{}",
            base_url.trim_end_matches('/'),
            issue_key
        );
        let resp = self
            .send(|| {
                self.get(&url)
                    .query(&[("fields", "issuelinks,subtasks")])
                    .send()
            })
            .await?;
        if !resp.status().is_success() {
            return Err(format!(
                "Could not load links for {}: {}",
                issue_key,
                resp.text().await.unwrap_or_default()
            ));
        }
        let data: IssueRelationsResponse = resp
            .json()
            .await
            .map_err(|e| format!("Parse links for {issue_key}: {e}"))?;
        Ok(parse_relations(issue_key, data.fields))
    }

    /// Create an issue link. `inward_key` / `outward_key` follow Jira link semantics.
    pub async fn link_issues(
        &self,
        base_url: &str,
        link_type_name: &str,
        inward_key: &str,
        outward_key: &str,
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

    pub async fn list_project_components(
        &self,
        base_url: &str,
        project_key: &str,
    ) -> Result<Vec<(String, String)>, String> {
        let url = format!(
            "{}/rest/api/3/project/{}/components",
            base_url.trim_end_matches('/'),
            project_key
        );
        let resp = self.send(|| self.get(&url).send()).await?;
        if !resp.status().is_success() {
            return Err(format!(
                "Components {}: {}",
                resp.status(),
                resp.text().await.unwrap_or_default()
            ));
        }
        let data: serde_json::Value = resp.json().await.map_err(|e| format!("Parse: {e}"))?;
        let mut out: Vec<(String, String)> = data
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|c| {
                        let id = c.get("id")?.as_str()?;
                        let name = c.get("name")?.as_str()?;
                        Some((id.to_string(), name.to_string()))
                    })
                    .collect()
            })
            .unwrap_or_default();
        out.sort_by(|a, b| a.1.cmp(&b.1));
        Ok(out)
    }

    pub async fn list_project_versions(
        &self,
        base_url: &str,
        project_key: &str,
    ) -> Result<Vec<(String, String)>, String> {
        let url = format!(
            "{}/rest/api/3/project/{}/versions",
            base_url.trim_end_matches('/'),
            project_key
        );
        let resp = self.send(|| self.get(&url).send()).await?;
        if !resp.status().is_success() {
            return Err(format!(
                "Versions {}: {}",
                resp.status(),
                resp.text().await.unwrap_or_default()
            ));
        }
        let data: serde_json::Value = resp.json().await.map_err(|e| format!("Parse: {e}"))?;
        let mut out: Vec<(String, String)> = data
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| {
                        let id = v.get("id")?.as_str()?;
                        let name = v.get("name")?.as_str()?;
                        Some((id.to_string(), name.to_string()))
                    })
                    .collect()
            })
            .unwrap_or_default();
        out.sort_by(|a, b| a.1.cmp(&b.1));
        Ok(out)
    }
}

fn parse_relations(issue_key: &str, fields: IssueRelationsFields) -> IssueRelations {
    let mut links = Vec::new();
    for link in fields.issuelinks {
        let type_name = link.link_type.name;
        if let Some(inward) = link.inward_issue {
            if inward.key != issue_key {
                links.push(IssueLinkView {
                    link_type: type_name.clone(),
                    direction: link.link_type.inward.clone(),
                    other_key: inward.key,
                    other_summary: inward.fields.summary,
                    other_status: inward.fields.status.name,
                });
                continue;
            }
        }
        if let Some(outward) = link.outward_issue {
            if outward.key != issue_key {
                links.push(IssueLinkView {
                    link_type: type_name,
                    direction: link.link_type.outward,
                    other_key: outward.key,
                    other_summary: outward.fields.summary,
                    other_status: outward.fields.status.name,
                });
            }
        }
    }
    let subtasks = fields
        .subtasks
        .into_iter()
        .map(|s| SubtaskView {
            key: s.key,
            summary: s.fields.summary,
            status: s.fields.status.name,
        })
        .collect();
    IssueRelations { links, subtasks }
}

/// Map picker selection to inward/outward keys for `link_issues`.
pub fn link_keys_for_picker(
    picker_index: usize,
    api_type: &str,
    source_key: &str,
    target_key: &str,
) -> (String, String) {
    if api_type == "Blocks" {
        if add_link_blocked_by(picker_index) {
            (source_key.to_string(), target_key.to_string())
        } else {
            (target_key.to_string(), source_key.to_string())
        }
    } else {
        (source_key.to_string(), target_key.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_issue_link_inward_other() {
        let json = serde_json::json!({
            "fields": {
                "issuelinks": [{
                    "type": { "name": "Relates", "inward": "relates to", "outward": "relates to" },
                    "inwardIssue": {
                        "key": "B-2",
                        "fields": { "summary": "Other", "status": { "name": "Open" } }
                    },
                    "outwardIssue": {
                        "key": "B-1",
                        "fields": { "summary": "Self", "status": { "name": "Done" } }
                    }
                }],
                "subtasks": [{
                    "key": "B-10",
                    "fields": { "summary": "Sub", "status": { "name": "To Do" } }
                }]
            }
        });
        let resp: IssueRelationsResponse = serde_json::from_value(json).unwrap();
        let rel = parse_relations("B-1", resp.fields);
        assert_eq!(rel.links.len(), 1);
        assert_eq!(rel.links[0].other_key, "B-2");
        assert_eq!(rel.subtasks[0].key, "B-10");
    }

    #[tokio::test]
    async fn link_issues_posts_payload() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/rest/api/3/issueLink"))
            .and(wiremock::matchers::body_json(serde_json::json!({
                "type": { "name": "Relates" },
                "inwardIssue": { "key": "A-1" },
                "outwardIssue": { "key": "A-2" },
            })))
            .respond_with(wiremock::ResponseTemplate::new(201))
            .mount(&server)
            .await;

        let client = JiraClient::new("u@example.com", "token", false);
        client
            .link_issues(&server.uri(), "Relates", "A-1", "A-2")
            .await
            .unwrap();
    }
}
