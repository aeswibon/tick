pub mod adf;
pub mod agile;
pub mod types;

use crate::config::Config;
use reqwest::Client;
use types::*;

use std::collections::HashMap;
use std::sync::Mutex;

pub struct JiraClient {
    pub http: Client,
    pub email: String,
    pub token: String,
    pub debug: bool,
    account_ids: Mutex<HashMap<String, String>>,
    priorities: Mutex<HashMap<String, Vec<(String, String)>>>,
}

impl JiraClient {
    pub fn new(email: &str, token: &str, debug: bool) -> Self {
        Self {
            http: Client::new(),
            email: email.to_string(),
            token: token.to_string(),
            debug,
            account_ids: Mutex::new(HashMap::new()),
            priorities: Mutex::new(HashMap::new()),
        }
    }

    pub async fn list_priorities(&self, base_url: &str) -> Result<Vec<(String, String)>, String> {
        let base = base_url.trim_end_matches('/');
        if let Ok(cache) = self.priorities.lock() {
            if let Some(list) = cache.get(base) {
                return Ok(list.clone());
            }
        }
        let url = format!("{base}/rest/api/3/priority");
        let resp = self
            .http
            .get(&url)
            .basic_auth(&self.email, Some(&self.token))
            .send()
            .await
            .map_err(|e| format!("HTTP error: {}", e))?;
        if !resp.status().is_success() {
            return Err(format!(
                "Priority API {}: {}",
                resp.status(),
                resp.text().await.unwrap_or_default()
            ));
        }
        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Parse error: {}", e))?;
        let list: Vec<(String, String)> = data
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|p| {
                        let id = p["id"].as_str()?;
                        let name = p["name"].as_str()?;
                        Some((id.to_string(), name.to_string()))
                    })
                    .collect()
            })
            .unwrap_or_default();
        if let Ok(mut cache) = self.priorities.lock() {
            cache.insert(base.to_string(), list.clone());
        }
        Ok(list)
    }

    pub async fn update_summary(
        &self,
        base_url: &str,
        key: &str,
        summary: &str,
    ) -> Result<(), String> {
        self.update_fields(base_url, key, serde_json::json!({ "summary": summary }))
            .await
    }

    pub async fn update_priority(
        &self,
        base_url: &str,
        key: &str,
        priority_name: &str,
    ) -> Result<(), String> {
        self.update_fields(
            base_url,
            key,
            serde_json::json!({ "priority": { "name": priority_name } }),
        )
        .await
    }

    pub async fn update_labels(
        &self,
        base_url: &str,
        key: &str,
        labels: &[String],
    ) -> Result<(), String> {
        self.update_fields(base_url, key, serde_json::json!({ "labels": labels }))
            .await
    }

    async fn update_fields(
        &self,
        base_url: &str,
        key: &str,
        fields: serde_json::Value,
    ) -> Result<(), String> {
        let url = format!(
            "{}/rest/api/3/issue/{}",
            base_url.trim_end_matches('/'),
            key
        );
        let resp = self
            .http
            .put(&url)
            .basic_auth(&self.email, Some(&self.token))
            .json(&serde_json::json!({ "fields": fields }))
            .send()
            .await
            .map_err(|e| format!("HTTP error: {}", e))?;
        if resp.status().is_success() {
            Ok(())
        } else {
            Err(format!(
                "Update failed: {}",
                resp.text().await.unwrap_or_default()
            ))
        }
    }

    pub async fn current_user_account_id(&self, base_url: &str) -> Result<String, String> {
        let base = base_url.trim_end_matches('/');
        if let Ok(cache) = self.account_ids.lock() {
            if let Some(id) = cache.get(base) {
                return Ok(id.clone());
            }
        }
        let url = format!("{base}/rest/api/3/myself");
        let resp = self
            .http
            .get(&url)
            .basic_auth(&self.email, Some(&self.token))
            .send()
            .await
            .map_err(|e| format!("HTTP error: {}", e))?;
        if !resp.status().is_success() {
            return Err(format!(
                "Myself API {}: {}",
                resp.status(),
                resp.text().await.unwrap_or_default()
            ));
        }
        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Parse error: {}", e))?;
        let id = body
            .get("accountId")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Myself API: missing accountId".to_string())?
            .to_string();
        if let Ok(mut cache) = self.account_ids.lock() {
            cache.insert(base.to_string(), id.clone());
        }
        Ok(id)
    }

    pub async fn assign_to_account(
        &self,
        base_url: &str,
        key: &str,
        account_id: &str,
    ) -> Result<(), String> {
        self.set_assignee(
            base_url,
            key,
            Some(serde_json::json!({ "accountId": account_id })),
        )
        .await
    }

    pub async fn unassign(&self, base_url: &str, key: &str) -> Result<(), String> {
        self.set_assignee(base_url, key, None).await
    }

    async fn set_assignee(
        &self,
        base_url: &str,
        key: &str,
        assignee: Option<serde_json::Value>,
    ) -> Result<(), String> {
        let url = format!(
            "{}/rest/api/3/issue/{}",
            base_url.trim_end_matches('/'),
            key
        );
        let resp = self
            .http
            .put(&url)
            .basic_auth(&self.email, Some(&self.token))
            .json(&serde_json::json!({
                "fields": { "assignee": assignee },
            }))
            .send()
            .await
            .map_err(|e| format!("HTTP error: {}", e))?;
        if resp.status().is_success() {
            Ok(())
        } else {
            Err(format!(
                "Assign failed: {}",
                resp.text().await.unwrap_or_default()
            ))
        }
    }

    pub async fn search_jql(
        &self,
        base_url: &str,
        jql: &str,
        max_results: u32,
    ) -> Result<Vec<String>, String> {
        let mut all_ids = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let url = format!("{}/rest/api/3/search/jql", base_url.trim_end_matches('/'));
            let mut body = serde_json::json!({
                "jql": jql,
                "maxResults": max_results,
            });
            if let Some(ref token) = next_token {
                body["nextPageToken"] = serde_json::Value::String(token.clone());
            }

            if self.debug {
                eprintln!("[DEBUG] POST {} (JQL: {})", url, jql);
            }

            let resp = self
                .http
                .post(&url)
                .basic_auth(&self.email, Some(&self.token))
                .json(&body)
                .send()
                .await
                .map_err(|e| format!("HTTP error: {}", e))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let body_text = resp.text().await.unwrap_or_default();
                return Err(format!("JQL search API {}: {}", status, body_text));
            }

            let text = resp
                .text()
                .await
                .map_err(|e| format!("Read error: {}", e))?;
            let data: JqlSearchResponse = serde_json::from_str(&text).map_err(|e| {
                format!("Parse error: {} | raw: {}", e, &text[..text.len().min(200)])
            })?;

            all_ids.extend(data.issues.into_iter().map(|i| i.id));

            match data.next_page_token {
                Some(token) if !token.is_empty() => next_token = Some(token),
                _ => break,
            }
        }

        Ok(all_ids)
    }

    pub async fn find_sprint_fields(
        &self,
        base_url: &str,
    ) -> Result<Vec<(String, String)>, String> {
        let url = format!("{}/rest/api/3/field", base_url.trim_end_matches('/'));
        let resp = self
            .http
            .get(&url)
            .basic_auth(&self.email, Some(&self.token))
            .send()
            .await
            .map_err(|e| format!("HTTP error: {}", e))?;
        if !resp.status().is_success() {
            return Err(format!(
                "Field API {}: {}",
                resp.status(),
                resp.text().await.unwrap_or_default()
            ));
        }
        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Parse error: {}", e))?;
        let list = data
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|f| {
                        let id = f["id"].as_str()?;
                        let name = f["name"].as_str()?;
                        if name.to_lowercase().contains("sprint") {
                            Some((id.to_string(), name.to_string()))
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        Ok(list)
    }

    pub async fn bulk_fetch(
        &self,
        base_url: &str,
        ids: &[String],
        sprint_field: Option<&str>,
    ) -> Result<Vec<BulkFetchIssue>, String> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let url = format!(
            "{}/rest/api/3/issue/bulkfetch",
            base_url.trim_end_matches('/')
        );
        let mut fields = vec![
            "issuetype",
            "status",
            "priority",
            "assignee",
            "reporter",
            "duedate",
            "created",
            "project",
            "summary",
            "description",
            "comment",
            "parent",
            "labels",
        ];
        if let Some(sf) = sprint_field {
            if !fields.contains(&sf) {
                fields.push(sf);
            }
        }

        let resp = self
            .http
            .post(&url)
            .basic_auth(&self.email, Some(&self.token))
            .json(&serde_json::json!({
                "issueIdsOrKeys": ids,
                "fields": fields,
            }))
            .send()
            .await
            .map_err(|e| format!("HTTP error: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Bulk fetch API {}: {}", status, body));
        }

        let text = resp
            .text()
            .await
            .map_err(|e| format!("Read error: {}", e))?;
        let data: BulkFetchResponse = serde_json::from_str(&text)
            .map_err(|e| format!("Parse error: {} | raw: {}", e, &text[..text.len().min(200)]))?;

        Ok(data.issues)
    }

    pub async fn get_transition_options(
        &self,
        base_url: &str,
        key: &str,
    ) -> Result<Vec<(String, String)>, String> {
        let url = format!(
            "{}/rest/api/3/issue/{}/transitions",
            base_url.trim_end_matches('/'),
            key
        );
        let resp = self
            .http
            .get(&url)
            .basic_auth(&self.email, Some(&self.token))
            .send()
            .await
            .map_err(|e| format!("HTTP error: {}", e))?;

        if !resp.status().is_success() {
            return Err(format!(
                "Transitions API {}: {}",
                resp.status(),
                resp.text().await.unwrap_or_default()
            ));
        }

        let text = resp
            .text()
            .await
            .map_err(|e| format!("Read error: {}", e))?;
        let data: serde_json::Value =
            serde_json::from_str(&text).map_err(|e| format!("Parse error: {}", e))?;
        Ok(data["transitions"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|t| {
                        let id = t["id"].as_str()?;
                        let name = t["name"].as_str()?;
                        Some((id.to_string(), name.to_string()))
                    })
                    .collect()
            })
            .unwrap_or_default())
    }

    pub async fn transition_issue(
        &self,
        base_url: &str,
        key: &str,
        transition_id: &str,
    ) -> Result<(), String> {
        let url = format!(
            "{}/rest/api/3/issue/{}/transitions",
            base_url.trim_end_matches('/'),
            key
        );
        let resp = self
            .http
            .post(&url)
            .basic_auth(&self.email, Some(&self.token))
            .json(&serde_json::json!({
                "transition": { "id": transition_id }
            }))
            .send()
            .await
            .map_err(|e| format!("HTTP error: {}", e))?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(format!(
                "Transition failed: {}",
                resp.text().await.unwrap_or_default()
            ))
        }
    }

    pub async fn add_comment(&self, base_url: &str, key: &str, body: &str) -> Result<(), String> {
        let url = format!(
            "{}/rest/api/3/issue/{}/comment",
            base_url.trim_end_matches('/'),
            key
        );
        let resp = self
            .http
            .post(&url)
            .basic_auth(&self.email, Some(&self.token))
            .json(&serde_json::json!({
                "body": adf::plain_text_body(body),
            }))
            .send()
            .await
            .map_err(|e| format!("HTTP error: {}", e))?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(format!(
                "Comment API {}: {}",
                resp.status(),
                resp.text().await.unwrap_or_default()
            ))
        }
    }

    pub async fn add_worklog(
        &self,
        base_url: &str,
        key: &str,
        time_spent: &str,
    ) -> Result<(), String> {
        let url = format!(
            "{}/rest/api/3/issue/{}/worklog",
            base_url.trim_end_matches('/'),
            key
        );
        let resp = self
            .http
            .post(&url)
            .basic_auth(&self.email, Some(&self.token))
            .json(&serde_json::json!({ "timeSpent": time_spent }))
            .send()
            .await
            .map_err(|e| format!("HTTP error: {}", e))?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(format!(
                "Worklog API {}: {}",
                resp.status(),
                resp.text().await.unwrap_or_default()
            ))
        }
    }
}

pub async fn fetch_all(
    client: &JiraClient,
    config: &Config,
    jql: &str,
) -> (Vec<Ticket>, Vec<String>) {
    let mut all = Vec::new();
    let mut errors = Vec::new();

    for site in &config.sites {
        if client.debug {
            eprintln!("[DEBUG] Processing site: {} ({})", site.name, site.base_url);
        }

        let ids = match client
            .search_jql(&site.base_url, jql, config.max_results)
            .await
        {
            Ok(k) => k,
            Err(e) => {
                errors.push(format!("{}: {}", site.name, e));
                continue;
            }
        };

        let sprint_field = site.sprint_field.as_deref();
        match client.bulk_fetch(&site.base_url, &ids, sprint_field).await {
            Ok(issues) => {
                for issue in issues {
                    all.push(Ticket::from_bulk_fetch(
                        issue,
                        &site.name,
                        &site.base_url,
                        sprint_field,
                    ));
                }
            }
            Err(e) => errors.push(format!("{}: {}", site.name, e)),
        }
    }

    (all, errors)
}

#[cfg(test)]
mod fetch_integration {
    use super::*;
    use crate::config::{Config, Site};

    fn test_config(base_url: &str) -> Config {
        Config {
            email: "user@example.com".into(),
            token: "token".into(),
            sites: vec![Site {
                name: "test".into(),
                base_url: base_url.into(),
                sprint_field: None,
                board_id: None,
                boards: Default::default(),
            }],
            columns: None,
            max_results: 50,
            page_size: 10,
            theme: "default".into(),
            views: Default::default(),
            notify_on_refresh: false,
            view_jql: Config::build_view_jql(&Default::default()),
        }
    }

    #[tokio::test]
    async fn fetch_all_maps_jql_and_bulk_fetch() {
        let server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/rest/api/3/search/jql"))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "issues": [{ "id": "10001" }]
                })),
            )
            .mount(&server)
            .await;

        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/rest/api/3/issue/bulkfetch"))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "issues": [{
                        "key": "DEMO-1",
                        "fields": {
                            "issuetype": { "name": "Task" },
                            "status": {
                                "name": "To Do",
                                "statusCategory": { "key": "new", "colorName": "blue-gray" }
                            },
                            "priority": { "name": "Medium" },
                            "assignee": { "displayName": "Alice" },
                            "reporter": { "displayName": "Bob" },
                            "created": "2026-01-01T00:00:00.000+0000",
                            "project": { "key": "DEMO" },
                            "summary": "Hello",
                            "labels": ["bug", "ui"]
                        }
                    }]
                })),
            )
            .mount(&server)
            .await;

        let config = test_config(&server.uri());
        let client = JiraClient::new("user@example.com", "token", false);
        let jql = config.jql_for(crate::view_mode::ViewMode::MyIssues);
        let (tickets, errors) = fetch_all(&client, &config, jql).await;

        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(tickets.len(), 1);
        assert_eq!(tickets[0].key, "DEMO-1");
        assert_eq!(tickets[0].site, "test");
        assert_eq!(tickets[0].summary, "Hello");
        assert_eq!(tickets[0].labels, vec!["bug", "ui"]);
    }
}

#[cfg(test)]
mod field_updates {
    use super::*;

    #[tokio::test]
    async fn update_summary_sends_put() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("PUT"))
            .and(wiremock::matchers::path("/rest/api/3/issue/DEMO-1"))
            .and(wiremock::matchers::body_json(
                serde_json::json!({ "fields": { "summary": "New title" } }),
            ))
            .respond_with(wiremock::ResponseTemplate::new(204))
            .mount(&server)
            .await;

        let client = JiraClient::new("u@example.com", "token", false);
        client
            .update_summary(&server.uri(), "DEMO-1", "New title")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn update_priority_sends_put() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("PUT"))
            .and(wiremock::matchers::path("/rest/api/3/issue/DEMO-1"))
            .and(wiremock::matchers::body_json(serde_json::json!({
                "fields": { "priority": { "name": "High" } }
            })))
            .respond_with(wiremock::ResponseTemplate::new(204))
            .mount(&server)
            .await;

        let client = JiraClient::new("u@example.com", "token", false);
        client
            .update_priority(&server.uri(), "DEMO-1", "High")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn update_labels_sends_put() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("PUT"))
            .and(wiremock::matchers::path("/rest/api/3/issue/DEMO-1"))
            .and(wiremock::matchers::body_json(serde_json::json!({
                "fields": { "labels": ["backend", "urgent"] }
            })))
            .respond_with(wiremock::ResponseTemplate::new(204))
            .mount(&server)
            .await;

        let client = JiraClient::new("u@example.com", "token", false);
        client
            .update_labels(
                &server.uri(),
                "DEMO-1",
                &["backend".into(), "urgent".into()],
            )
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn list_priorities_parses_response() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/rest/api/3/priority"))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!([
                    { "id": "1", "name": "High" },
                    { "id": "2", "name": "Low" }
                ])),
            )
            .mount(&server)
            .await;

        let client = JiraClient::new("u@example.com", "token", false);
        let list = client.list_priorities(&server.uri()).await.unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].1, "High");
    }
}
