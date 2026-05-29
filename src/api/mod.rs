pub mod adf;
pub mod types;

use crate::config::Config;
use reqwest::Client;
use types::*;

pub struct JiraClient {
    pub http: Client,
    pub email: String,
    pub token: String,
    pub debug: bool,
}

impl JiraClient {
    pub fn new(email: &str, token: &str, debug: bool) -> Self {
        Self {
            http: Client::new(),
            email: email.to_string(),
            token: token.to_string(),
            debug,
        }
    }

    pub async fn search_jql(&self, base_url: &str, jql: &str, max_results: u32) -> Result<Vec<String>, String> {
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

            let text = resp.text().await.map_err(|e| format!("Read error: {}", e))?;
            let data: JqlSearchResponse = serde_json::from_str(&text)
                .map_err(|e| format!("Parse error: {} | raw: {}", e, &text[..text.len().min(200)]))?;

            all_ids.extend(data.issues.into_iter().map(|i| i.id));

            match data.next_page_token {
                Some(token) if !token.is_empty() => next_token = Some(token),
                _ => break,
            }
        }

        Ok(all_ids)
    }

    pub async fn bulk_fetch(&self, base_url: &str, ids: &[String]) -> Result<Vec<BulkFetchIssue>, String> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let url = format!("{}/rest/api/3/issue/bulkfetch", base_url.trim_end_matches('/'));
        let fields = [
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
        ];

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

        let text = resp.text().await.map_err(|e| format!("Read error: {}", e))?;
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

        let text = resp.text().await.map_err(|e| format!("Read error: {}", e))?;
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

    pub async fn transition_issue(&self, base_url: &str, key: &str, transition_id: &str) -> Result<(), String> {
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

    pub async fn add_worklog(&self, base_url: &str, key: &str, time_spent: &str) -> Result<(), String> {
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

        let ids = match client.search_jql(&site.base_url, jql, config.max_results).await {
            Ok(k) => k,
            Err(e) => {
                errors.push(format!("{}: {}", site.name, e));
                continue;
            }
        };

        match client.bulk_fetch(&site.base_url, &ids).await {
            Ok(issues) => {
                for issue in issues {
                    all.push(Ticket::from_bulk_fetch(
                        issue,
                        &site.name,
                        &site.base_url,
                    ));
                }
            }
            Err(e) => errors.push(format!("{}: {}", site.name, e)),
        }
    }

    (all, errors)
}
