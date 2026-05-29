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
        let url = format!("{}/rest/api/3/search/jql", base_url.trim_end_matches('/'));

        if self.debug {
            eprintln!("[DEBUG] POST {} (JQL: {})", url, jql);
        }

        let resp = self.http
            .post(&url)
            .basic_auth(&self.email, Some(&self.token))
            .json(&serde_json::json!({
                "jql": jql,
                "maxResults": max_results,
            }))
            .send()
            .await
            .map_err(|e| format!("HTTP error: {}", e))?;

        if self.debug {
            eprintln!("[DEBUG] Response status: {}", resp.status());
        }

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            if self.debug {
                eprintln!("[DEBUG] Error body: {}", body);
            }
            return Err(format!("JQL search API {}: {}", status, body));
        }

        let text = resp.text().await.map_err(|e| format!("Read error: {}", e))?;
        if self.debug {
            eprintln!("[DEBUG] Response body (first 500 chars): {}", &text[..text.len().min(500)]);
        }

        let data: JqlSearchResponse = serde_json::from_str(&text)
            .map_err(|e| format!("Parse error: {} | raw: {}", e, &text[..text.len().min(200)]))?;

        Ok(data.issues.into_iter().map(|i| i.id).collect())
    }

    pub async fn bulk_fetch(&self, base_url: &str, ids: &[String]) -> Result<Vec<BulkFetchIssue>, String> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let url = format!("{}/rest/api/3/issue/bulkfetch", base_url.trim_end_matches('/'));
        let fields = ["issuetype", "status", "priority", "assignee", "reporter", "duedate", "created", "project", "summary", "description", "comment", "parent"];

        if self.debug {
            eprintln!("[DEBUG] POST {} (ids: {:?})", url, ids);
        }

        let resp = self.http
            .post(&url)
            .basic_auth(&self.email, Some(&self.token))
            .json(&serde_json::json!({
                "issueIdsOrKeys": ids,
                "fields": fields,
            }))
            .send()
            .await
            .map_err(|e| format!("HTTP error: {}", e))?;

        if self.debug {
            eprintln!("[DEBUG] Response status: {}", resp.status());
        }

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            if self.debug {
                eprintln!("[DEBUG] Error body: {}", body);
            }
            return Err(format!("Bulk fetch API {}: {}", status, body));
        }

        let text = resp.text().await.map_err(|e| format!("Read error: {}", e))?;
        if self.debug {
            eprintln!("[DEBUG] Response body (first 500 chars): {}", &text[..text.len().min(500)]);
        }

        let data: BulkFetchResponse = serde_json::from_str(&text)
            .map_err(|e| format!("Parse error: {} | raw: {}", e, &text[..text.len().min(200)]))?;

        Ok(data.issues)
    }

    /// Fetch available transitions as (id, name) pairs for the transition picker
    pub async fn get_transition_options(&self, base_url: &str, key: &str) -> Result<Vec<(String, String)>, String> {
        let url = format!("{}/rest/api/3/issue/{}/transitions", base_url.trim_end_matches('/'), key);
        let resp = self.http
            .get(&url)
            .basic_auth(&self.email, Some(&self.token))
            .send()
            .await
            .map_err(|e| format!("HTTP error: {}", e))?;

        if !resp.status().is_success() {
            return Err(format!("Transitions API {}: {}", resp.status(), resp.text().await.unwrap_or_default()));
        }

        let text = resp.text().await.map_err(|e| format!("Read error: {}", e))?;
        let data: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("Parse error: {}", e))?;
        let pairs = data["transitions"].as_array()
            .map(|arr| {
                arr.iter().filter_map(|t| {
                    let id = t["id"].as_str()?;
                    let name = t["name"].as_str()?;
                    Some((id.to_string(), name.to_string()))
                }).collect()
            })
            .unwrap_or_default();
        Ok(pairs)
    }

    pub async fn add_comment(&self, base_url: &str, key: &str, body: &str) -> Result<(), String> {
        let url = format!("{}/rest/api/3/issue/{}/comment", base_url.trim_end_matches('/'), key);
        let resp = self.http
            .post(&url)
            .basic_auth(&self.email, Some(&self.token))
            .json(&serde_json::json!({ "body": body }))
            .send()
            .await
            .map_err(|e| format!("HTTP error: {}", e))?;

        if !resp.status().is_success() {
            return Err(format!("Comment API {}: {}", resp.status(), resp.text().await.unwrap_or_default()));
        }
        Ok(())
    }

    pub async fn add_worklog(&self, base_url: &str, key: &str, time_spent: &str) -> Result<(), String> {
        let url = format!("{}/rest/api/3/issue/{}/worklog", base_url.trim_end_matches('/'), key);
        let resp = self.http
            .post(&url)
            .basic_auth(&self.email, Some(&self.token))
            .json(&serde_json::json!({ "timeSpent": time_spent }))
            .send()
            .await
            .map_err(|e| format!("HTTP error: {}", e))?;

        if !resp.status().is_success() {
            return Err(format!("Worklog API {}: {}", resp.status(), resp.text().await.unwrap_or_default()));
        }
        Ok(())
    }
}

pub async fn fetch_all(config: &Config, debug: bool, jql: &str) -> (Vec<Ticket>, Vec<String>) {
    let client = JiraClient::new(&config.email, &config.token, debug);
    let mut all = Vec::new();
    let mut errors = Vec::new();

    for site in &config.sites {
        if debug {
            eprintln!("[DEBUG] Processing site: {} ({})", site.name, site.base_url);
        }

        let ids = match client.search_jql(&site.base_url, jql, config.max_results).await {
            Ok(k) => k,
            Err(e) => {
                errors.push(format!("{}: {}", site.name, e));
                continue;
            }
        };

        if debug {
            eprintln!("[DEBUG] Found {} ids for {}", ids.len(), site.name);
        }

        match client.bulk_fetch(&site.base_url, &ids).await {
            Ok(issues) => {
                if debug {
                    eprintln!("[DEBUG] Bulk fetched {} issues for {}", issues.len(), site.name);
                }
                for issue in issues {
                    all.push(Ticket::from_bulk_fetch(
                        issue, &site.name, &site.base_url,
                    ));
                }
            }
            Err(e) => {
                errors.push(format!("{}: {}", site.name, e));
            }
        }
    }

    (all, errors)
}
