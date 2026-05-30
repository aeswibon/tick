pub mod adf;
pub mod adf_export;
pub mod agile;
pub mod assignable_users;
pub mod create;
pub mod issue_relations;
pub mod jira_error;
pub mod markdown;
pub mod retry;
pub mod transition_fields;
pub mod types;

use crate::auth::Auth;
use crate::config::{Config, Site};
use reqwest::Client;
use reqwest::StatusCode;
use types::*;

use std::collections::HashMap;
use std::sync::Mutex;

/// Failed transition POST; `field_errors` lists Jira `errors` keys when present.
#[derive(Debug, Clone)]
pub struct TransitionError {
    pub message: String,
    pub field_errors: Vec<(String, String)>,
}

pub use create::{CreateDraft, CreateError};

pub struct JiraClient {
    pub http: Client,
    auth: Auth,
    pub debug: bool,
    account_ids: Mutex<HashMap<String, String>>,
    priorities: Mutex<HashMap<String, Vec<(String, String)>>>,
    resolutions: Mutex<HashMap<String, Vec<(String, String)>>>,
    /// Full assignable-user catalog per `base_url|issue_key`.
    assignable_users: Mutex<HashMap<String, Vec<(String, String)>>>,
}

impl JiraClient {
    pub fn new(email: &str, token: &str, debug: bool) -> Self {
        Self::with_auth(Auth::basic(email, token), debug)
    }

    pub fn with_auth(auth: Auth, debug: bool) -> Self {
        Self {
            http: Client::new(),
            auth,
            debug,
            account_ids: Mutex::new(HashMap::new()),
            priorities: Mutex::new(HashMap::new()),
            resolutions: Mutex::new(HashMap::new()),
            assignable_users: Mutex::new(HashMap::new()),
        }
    }

    pub async fn from_config(config: &Config, debug: bool) -> Result<Self, String> {
        let auth = config.resolve_auth().await?;
        Ok(Self::with_auth(auth, debug))
    }

    pub fn email(&self) -> &str {
        self.auth.email()
    }

    fn authed(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        self.auth.apply(req)
    }

    pub(crate) fn get(&self, url: &str) -> reqwest::RequestBuilder {
        self.authed(self.http.get(url))
    }

    pub(crate) fn put(&self, url: &str) -> reqwest::RequestBuilder {
        self.authed(self.http.put(url))
    }

    pub(crate) fn post(&self, url: &str) -> reqwest::RequestBuilder {
        self.authed(self.http.post(url))
    }

    pub(crate) fn delete(&self, url: &str) -> reqwest::RequestBuilder {
        self.authed(self.http.delete(url))
    }

    pub(crate) async fn send<F, Fut>(&self, build: F) -> Result<reqwest::Response, String>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<reqwest::Response, reqwest::Error>>,
    {
        retry::with_retry(build).await
    }

    pub async fn list_priorities(&self, base_url: &str) -> Result<Vec<(String, String)>, String> {
        let base = base_url.trim_end_matches('/');
        if let Ok(cache) = self.priorities.lock() {
            if let Some(list) = cache.get(base) {
                return Ok(list.clone());
            }
        }
        let url = format!("{base}/rest/api/3/priority");
        let resp = self.send(|| self.get(&url).send()).await?;
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

    pub async fn update_description(
        &self,
        base_url: &str,
        key: &str,
        text: &str,
        mentions: &[(String, String)],
    ) -> Result<(), String> {
        let body = markdown::to_adf(text, mentions);
        self.update_fields(base_url, key, serde_json::json!({ "description": body }))
            .await
    }

    /// Set or clear due date (`YYYY-MM-DD`). `None` clears the field.
    pub async fn update_due_date(
        &self,
        base_url: &str,
        key: &str,
        due_date: Option<chrono::NaiveDate>,
    ) -> Result<(), String> {
        let value = match due_date {
            Some(d) => serde_json::json!(d.format("%Y-%m-%d").to_string()),
            None => serde_json::Value::Null,
        };
        self.update_fields(base_url, key, serde_json::json!({ "duedate": value }))
            .await
    }

    /// Add the current user as a watcher on the issue.
    pub async fn watch_issue(&self, base_url: &str, key: &str) -> Result<(), String> {
        let account_id = self.current_user_account_id(base_url).await?;
        let url = format!(
            "{}/rest/api/3/issue/{}/watchers",
            base_url.trim_end_matches('/'),
            key
        );
        let resp = self
            .send(|| {
                self.post(&url)
                    .json(&serde_json::json!({ "accountId": account_id }))
                    .send()
            })
            .await?;
        if resp.status().is_success() {
            Ok(())
        } else {
            Err(format!(
                "Watch failed: {}",
                resp.text().await.unwrap_or_default()
            ))
        }
    }

    /// Remove the current user from watchers.
    pub async fn unwatch_issue(&self, base_url: &str, key: &str) -> Result<(), String> {
        let account_id = self.current_user_account_id(base_url).await?;
        let url = format!(
            "{}/rest/api/3/issue/{}/watchers",
            base_url.trim_end_matches('/'),
            key
        );
        let resp = self
            .send(|| {
                self.delete(&url)
                    .query(&[("accountId", account_id.as_str())])
                    .send()
            })
            .await?;
        if resp.status().is_success() {
            Ok(())
        } else {
            Err(format!(
                "Unwatch failed: {}",
                resp.text().await.unwrap_or_default()
            ))
        }
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
            .send(|| {
                self.put(&url)
                    .json(&serde_json::json!({ "fields": fields }))
                    .send()
            })
            .await?;
        if resp.status().is_success() {
            Ok(())
        } else {
            Err(format!(
                "Update failed: {}",
                resp.text().await.unwrap_or_default()
            ))
        }
    }

    /// `GET /rest/api/3/issue/{key}` — true when the issue exists (or is forbidden but present).
    pub async fn issue_exists(&self, base_url: &str, key: &str) -> bool {
        let base = base_url.trim_end_matches('/');
        let url = format!("{base}/rest/api/3/issue/{key}?fields=key");
        let resp = match self.send(|| self.get(&url).send()).await {
            Ok(r) => r,
            Err(_) => return false,
        };
        let status = resp.status();
        status.is_success() || status == StatusCode::FORBIDDEN
    }

    /// Probe sites in config order; return browse URL for the first match.
    pub async fn find_issue_browse_url(&self, sites: &[Site], key: &str) -> Option<String> {
        for site in sites {
            if self.issue_exists(&site.base_url, key).await {
                let base = site.base_url.trim_end_matches('/');
                return Some(format!("{base}/browse/{key}"));
            }
        }
        None
    }

    pub async fn current_user_account_id(&self, base_url: &str) -> Result<String, String> {
        let base = base_url.trim_end_matches('/');
        if let Ok(cache) = self.account_ids.lock() {
            if let Some(id) = cache.get(base) {
                return Ok(id.clone());
            }
        }
        let url = format!("{base}/rest/api/3/myself");
        let resp = self.send(|| self.get(&url).send()).await?;
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
        let payload = serde_json::json!({ "fields": { "assignee": assignee } });
        let resp = self.send(|| self.put(&url).json(&payload).send()).await?;
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

            let resp = self.send(|| self.post(&url).json(&body).send()).await?;

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
        let resp = self.send(|| self.get(&url).send()).await?;
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
        custom_field_ids: &[String],
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
        for cf in custom_field_ids {
            if !fields.iter().any(|f| f == cf) {
                fields.push(cf.as_str());
            }
        }

        let payload = serde_json::json!({
            "issueIdsOrKeys": ids,
            "fields": fields,
        });
        let resp = self.send(|| self.post(&url).json(&payload).send()).await?;

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

    /// List workflow transitions available for the issue (status changes go through these).
    pub async fn get_workflow_transitions(
        &self,
        base_url: &str,
        key: &str,
    ) -> Result<Vec<types::WorkflowTransition>, String> {
        let url = format!(
            "{}/rest/api/3/issue/{}/transitions?expand=transitions.fields",
            base_url.trim_end_matches('/'),
            key
        );
        let resp = self.send(|| self.get(&url).send()).await?;

        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(jira_error::format_response_error(status, &body));
        }

        let data: serde_json::Value =
            serde_json::from_str(&body).map_err(|e| format!("Parse transitions: {e}"))?;
        Ok(parse_workflow_transitions(&data))
    }

    /// Fetch one transition with full screen field metadata (Jira often omits fields on the list call).
    pub async fn get_transition_detail(
        &self,
        base_url: &str,
        key: &str,
        transition_id: &str,
    ) -> Result<types::WorkflowTransition, String> {
        let url = format!(
            "{}/rest/api/3/issue/{}/transitions?transitionId={}&expand=transitions.fields",
            base_url.trim_end_matches('/'),
            key,
            transition_id
        );
        let resp = self.send(|| self.get(&url).send()).await?;
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(jira_error::format_response_error(status, &body));
        }
        let data: serde_json::Value =
            serde_json::from_str(&body).map_err(|e| format!("Parse transition detail: {e}"))?;
        parse_single_transition(&data)
            .ok_or_else(|| format!("Transition {transition_id} not returned by Jira for {key}"))
    }

    /// Project resolutions (cached per site for the session).
    pub async fn list_resolutions(&self, base_url: &str) -> Result<Vec<(String, String)>, String> {
        let base = base_url.trim_end_matches('/');
        if let Ok(cache) = self.resolutions.lock() {
            if let Some(list) = cache.get(base) {
                return Ok(list.clone());
            }
        }
        let url = format!("{base}/rest/api/3/resolution");
        let resp = self.send(|| self.get(&url).send()).await?;
        if !resp.status().is_success() {
            return Err(format!(
                "Resolution list {}: {}",
                resp.status(),
                resp.text().await.unwrap_or_default()
            ));
        }
        let data: serde_json::Value = resp.json().await.map_err(|e| format!("Parse error: {e}"))?;
        let list: Vec<(String, String)> = data
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|r| {
                        let id = r["id"].as_str()?.to_string();
                        let name = r["name"].as_str()?.to_string();
                        Some((id, name))
                    })
                    .collect()
            })
            .unwrap_or_default();
        if let Ok(mut cache) = self.resolutions.lock() {
            cache.insert(base.to_string(), list.clone());
        }
        Ok(list)
    }

    /// Preload resolution + priority catalogs (parallel, cached) before field prompts.
    pub async fn warm_site_field_catalogs(&self, base_url: &str) {
        let base = base_url.trim_end_matches('/');
        let need_res = self
            .resolutions
            .lock()
            .ok()
            .is_none_or(|c| !c.contains_key(base));
        let need_pri = self
            .priorities
            .lock()
            .ok()
            .is_none_or(|c| !c.contains_key(base));
        if !need_res && !need_pri {
            return;
        }
        let (res, pri) = tokio::join!(
            async {
                if need_res {
                    self.list_resolutions(base_url).await.ok()
                } else {
                    None
                }
            },
            async {
                if need_pri {
                    self.list_priorities(base_url).await.ok()
                } else {
                    None
                }
            }
        );
        let _ = (res, pri);
    }

    pub async fn transition_issue(
        &self,
        base_url: &str,
        key: &str,
        transition: &types::WorkflowTransition,
        fields: &HashMap<String, serde_json::Value>,
    ) -> Result<(), TransitionError> {
        let url = format!(
            "{}/rest/api/3/issue/{}/transitions",
            base_url.trim_end_matches('/'),
            key
        );
        let mut payload = serde_json::json!({ "transition": { "id": transition.id } });
        if !fields.is_empty() {
            payload["fields"] = serde_json::json!(fields);
        }
        let resp = self
            .send(|| self.post(&url).json(&payload).send())
            .await
            .map_err(|e| TransitionError {
                message: e,
                field_errors: Vec::new(),
            })?;

        let status = resp.status();
        if status.is_success() {
            return Ok(());
        }
        let body = resp.text().await.unwrap_or_default();
        let detail = jira_error::format_response_error(status, &body);
        let label = if transition.to_status.is_empty() {
            transition.name.clone()
        } else {
            format!("{} → {}", transition.name, transition.to_status)
        };
        Err(TransitionError {
            message: format!("Cannot apply \"{label}\": {detail}"),
            field_errors: jira_error::field_errors(&body),
        })
    }

    async fn fetch_assignable_users_api(
        &self,
        base_url: &str,
        issue_key: &str,
        query: &str,
        max_results: &str,
    ) -> Result<Vec<(String, String)>, String> {
        let url = format!(
            "{}/rest/api/3/user/assignable/search",
            base_url.trim_end_matches('/')
        );
        let project_only = issue_key.strip_suffix(".__CREATE__");
        let resp = self
            .send(|| {
                let mut req = self
                    .get(&url)
                    .query(&[("query", query), ("maxResults", max_results)]);
                if let Some(project) = project_only {
                    req = req.query(&[("project", project)]);
                } else {
                    req = req.query(&[("issueKey", issue_key)]);
                }
                req.send()
            })
            .await?;
        if !resp.status().is_success() {
            return Err(format!(
                "User search {}: {}",
                resp.status(),
                resp.text().await.unwrap_or_default()
            ));
        }
        let data: serde_json::Value = resp.json().await.map_err(|e| format!("Parse error: {e}"))?;
        Ok(parse_user_list(&data))
    }

    /// Load assignable users for an issue into the session cache.
    ///
    /// Initial load uses `api_query` `""` (up to 100). On refresh, fetches with `api_query`
    /// (often the footer filter) and **merges** into the existing catalog.
    pub async fn ensure_assignable_users(
        &self,
        base_url: &str,
        issue_key: &str,
        api_query: &str,
        force_refresh: bool,
    ) -> Result<Vec<(String, String)>, String> {
        let key = assignable_users::cache_key(base_url, issue_key);
        let existing = if force_refresh {
            self.assignable_users
                .lock()
                .ok()
                .and_then(|c| c.get(&key).cloned())
                .unwrap_or_default()
        } else if let Ok(cache) = self.assignable_users.lock() {
            if let Some(list) = cache.get(&key) {
                return Ok(list.clone());
            }
            Vec::new()
        } else {
            Vec::new()
        };

        let fetched = self
            .fetch_assignable_users_api(
                base_url,
                issue_key,
                api_query,
                assignable_users::CATALOG_MAX,
            )
            .await?;

        let list = if force_refresh {
            assignable_users::merge_users(&existing, &fetched)
        } else {
            fetched
        };

        if let Ok(mut cache) = self.assignable_users.lock() {
            cache.insert(key, list.clone());
        }
        Ok(list)
    }

    /// Filter cached assignable users (loads catalog on first use).
    pub async fn filter_assignable_users(
        &self,
        base_url: &str,
        issue_key: &str,
        query: &str,
    ) -> Result<Vec<(String, String)>, String> {
        let catalog = self
            .ensure_assignable_users(base_url, issue_key, "", false)
            .await?;
        Ok(assignable_users::filter_users(&catalog, query))
    }

    pub async fn search_assignable_users(
        &self,
        base_url: &str,
        issue_key: &str,
        query: &str,
    ) -> Result<Vec<(String, String)>, String> {
        self.filter_assignable_users(base_url, issue_key, query)
            .await
    }
}

fn parse_user_list(data: &serde_json::Value) -> Vec<(String, String)> {
    data.as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|u| {
                    let id = u["accountId"].as_str()?;
                    let name = u["displayName"].as_str()?;
                    Some((id.to_string(), name.to_string()))
                })
                .collect()
        })
        .unwrap_or_default()
}

impl JiraClient {
    pub async fn add_comment(
        &self,
        base_url: &str,
        key: &str,
        body: &str,
        mentions: &[(String, String)],
    ) -> Result<(), String> {
        let url = format!(
            "{}/rest/api/3/issue/{}/comment",
            base_url.trim_end_matches('/'),
            key
        );
        let adf_body = markdown::to_adf(body, mentions);
        let payload = serde_json::json!({ "body": adf_body });
        let resp = self.send(|| self.post(&url).json(&payload).send()).await?;

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
            .send(|| {
                self.post(&url)
                    .json(&serde_json::json!({ "timeSpent": time_spent }))
                    .send()
            })
            .await?;

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

fn parse_workflow_transitions(data: &serde_json::Value) -> Vec<types::WorkflowTransition> {
    data["transitions"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|t| {
                    let id = t["id"].as_str()?.to_string();
                    let name = t["name"].as_str()?.to_string();
                    let to_status = t
                        .get("to")
                        .and_then(|to| to.get("name"))
                        .and_then(|n| n.as_str())
                        .unwrap_or("")
                        .to_string();
                    let required_fields =
                        transition_fields::parse_transition_screen_fields(t.get("fields"));
                    Some(types::WorkflowTransition {
                        id,
                        name,
                        to_status,
                        required_fields,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn parse_single_transition(data: &serde_json::Value) -> Option<types::WorkflowTransition> {
    data["transitions"].as_array()?.first().and_then(|t| {
        let id = t["id"].as_str()?.to_string();
        let name = t["name"].as_str()?.to_string();
        let to_status = t
            .get("to")
            .and_then(|to| to.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("")
            .to_string();
        let mut required_fields =
            transition_fields::parse_transition_screen_fields(t.get("fields"));
        if required_fields.is_empty() {
            if let Some(res) =
                transition_fields::infer_resolution_if_done_transition(&name, &to_status)
            {
                required_fields.push(res);
            }
        }
        Some(types::WorkflowTransition {
            id,
            name,
            to_status,
            required_fields,
        })
    })
}

pub async fn enrich_transition_fields(
    client: &JiraClient,
    base_url: &str,
    project_key: Option<&str>,
    transition: &mut types::WorkflowTransition,
) {
    let needs_resolution = transition
        .required_fields
        .iter()
        .any(|f| f.id == "resolution" && f.options.is_empty());
    let needs_priority = transition.required_fields.iter().any(|f| {
        (f.id == "priority" || f.system == "priority")
            && f.options.is_empty()
            && f.kind == transition_fields::TransitionFieldKind::Picker
    });
    let needs_components = transition.required_fields.iter().any(|f| {
        f.id == "components"
            && f.options.is_empty()
            && f.kind == transition_fields::TransitionFieldKind::MultiPicker
    });
    let needs_versions = transition.required_fields.iter().any(|f| {
        f.id == "fixVersions"
            && f.options.is_empty()
            && f.kind == transition_fields::TransitionFieldKind::MultiPicker
    });
    if !needs_resolution && !needs_priority && !needs_components && !needs_versions {
        return;
    }

    let pk = project_key.unwrap_or("");
    let (resolutions, priorities, components, versions) = tokio::join!(
        async {
            if needs_resolution {
                client.list_resolutions(base_url).await.ok()
            } else {
                None
            }
        },
        async {
            if needs_priority {
                client.list_priorities(base_url).await.ok()
            } else {
                None
            }
        },
        async {
            if needs_components && !pk.is_empty() {
                client.list_project_components(base_url, pk).await.ok()
            } else {
                None
            }
        },
        async {
            if needs_versions && !pk.is_empty() {
                client.list_project_versions(base_url, pk).await.ok()
            } else {
                None
            }
        },
    );

    if let Some(opts) = resolutions {
        for field in &mut transition.required_fields {
            if field.id == "resolution" && field.options.is_empty() {
                field.options = opts.clone();
            }
        }
    }
    if let Some(opts) = priorities {
        for field in &mut transition.required_fields {
            if (field.id == "priority" || field.system == "priority") && field.options.is_empty() {
                field.options = opts.clone();
            }
        }
    }
    if let Some(opts) = components {
        for field in &mut transition.required_fields {
            if field.id == "components" && field.options.is_empty() {
                field.options = opts.clone();
            }
        }
    }
    if let Some(opts) = versions {
        for field in &mut transition.required_fields {
            if field.id == "fixVersions" && field.options.is_empty() {
                field.options = opts.clone();
            }
        }
    }
}

pub async fn fetch_all(
    client: &JiraClient,
    config: &Config,
    jql: &str,
    site_filter: Option<&str>,
    custom_field_ids: &[String],
) -> (Vec<Ticket>, Vec<String>) {
    let mut all = Vec::new();
    let mut errors = Vec::new();
    let cf_refs: Vec<&str> = custom_field_ids.iter().map(String::as_str).collect();

    for site in config.sites_for_fetch(site_filter) {
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
        match client
            .bulk_fetch(&site.base_url, &ids, sprint_field, custom_field_ids)
            .await
        {
            Ok(issues) => {
                for issue in issues {
                    all.push(Ticket::from_bulk_fetch(
                        issue,
                        &site.name,
                        &site.base_url,
                        sprint_field,
                        &cf_refs,
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
                ..Default::default()
            }],
            columns: None,
            max_results: 50,
            page_size: 10,
            theme: "default".into(),
            views: Default::default(),
            notify_on_refresh: false,
            auth: Default::default(),
            oauth: Default::default(),
            create: Default::default(),
            hooks: Default::default(),
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
        let (tickets, errors) = fetch_all(&client, &config, jql, None, &[]).await;

        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(tickets.len(), 1);
        assert_eq!(tickets[0].key, "DEMO-1");
        assert_eq!(tickets[0].site, "test");
        assert_eq!(tickets[0].summary, "Hello");
        assert_eq!(tickets[0].labels, vec!["bug", "ui"]);
    }

    #[tokio::test]
    async fn fetch_all_keeps_successful_sites_when_another_site_fails() {
        let failing = wiremock::MockServer::start().await;
        let passing = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/rest/api/3/search/jql"))
            .respond_with(wiremock::ResponseTemplate::new(401).set_body_string("unauthorized"))
            .mount(&failing)
            .await;

        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/rest/api/3/search/jql"))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "issues": [{ "id": "20001" }]
                })),
            )
            .mount(&passing)
            .await;

        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/rest/api/3/issue/bulkfetch"))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "issues": [{
                        "key": "PASS-1",
                        "fields": {
                            "issuetype": { "name": "Bug" },
                            "status": {
                                "name": "In Progress",
                                "statusCategory": { "key": "indeterminate", "colorName": "yellow" }
                            },
                            "priority": null,
                            "assignee": null,
                            "reporter": null,
                            "duedate": null,
                            "created": "2026-01-01T00:00:00.000+0000",
                            "project": { "key": "PASS" },
                            "summary": "Still fetched",
                            "labels": null
                        }
                    }]
                })),
            )
            .mount(&passing)
            .await;

        let mut config = test_config(&failing.uri());
        config.sites[0].name = "broken".into();
        config.sites.push(Site {
            name: "healthy".into(),
            base_url: passing.uri(),
            ..Default::default()
        });

        let client = JiraClient::new("user@example.com", "token", false);
        let jql = config.jql_for(crate::view_mode::ViewMode::MyIssues);
        let (tickets, errors) = fetch_all(&client, &config, jql, None, &[]).await;

        assert_eq!(tickets.len(), 1);
        assert_eq!(tickets[0].key, "PASS-1");
        assert_eq!(tickets[0].site, "healthy");
        assert_eq!(tickets[0].priority, "");
        assert_eq!(tickets[0].assignee, "");
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("broken:"));
        assert!(errors[0].contains("401"));
    }
}

#[cfg(test)]
mod field_updates {
    use super::*;

    #[tokio::test]
    async fn find_issue_browse_url_probes_sites_in_order() {
        let first = wiremock::MockServer::start().await;
        let second = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/rest/api/3/issue/FOO-1"))
            .respond_with(wiremock::ResponseTemplate::new(404))
            .mount(&first)
            .await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/rest/api/3/issue/FOO-1"))
            .respond_with(
                wiremock::ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "key": "FOO-1" })),
            )
            .mount(&second)
            .await;

        let sites = vec![
            crate::config::Site {
                name: "a".into(),
                base_url: first.uri(),
                ..Default::default()
            },
            crate::config::Site {
                name: "b".into(),
                base_url: second.uri(),
                ..Default::default()
            },
        ];
        let client = JiraClient::new("u@example.com", "token", false);
        let url = client.find_issue_browse_url(&sites, "FOO-1").await.unwrap();
        assert_eq!(url, format!("{}/browse/FOO-1", second.uri()));
    }

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
    async fn update_description_sends_put() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("PUT"))
            .and(wiremock::matchers::path("/rest/api/3/issue/DEMO-1"))
            .respond_with(wiremock::ResponseTemplate::new(204))
            .mount(&server)
            .await;

        let client = JiraClient::new("u@example.com", "token", false);
        client
            .update_description(&server.uri(), "DEMO-1", "# Hello\n\nWorld", &[])
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn search_assignable_users_parses_response() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path(
                "/rest/api/3/user/assignable/search",
            ))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!([
                    { "accountId": "acc-1", "displayName": "Alice" },
                    { "accountId": "acc-2", "displayName": "Bob" }
                ])),
            )
            .mount(&server)
            .await;

        let client = JiraClient::new("u@example.com", "token", false);
        let catalog = client
            .ensure_assignable_users(&server.uri(), "DEMO-1", "", false)
            .await
            .unwrap();
        assert_eq!(catalog.len(), 2);

        let users = client
            .filter_assignable_users(&server.uri(), "DEMO-1", "al")
            .await
            .unwrap();
        assert_eq!(users.len(), 1);
        assert_eq!(users[0], ("acc-1".into(), "Alice".into()));

        // Refresh merges (same response still dedupes to 2).
        let merged = client
            .ensure_assignable_users(&server.uri(), "DEMO-1", "", true)
            .await
            .unwrap();
        assert_eq!(merged.len(), 2);

        // Second load uses cache (no extra HTTP).
        let again = client
            .ensure_assignable_users(&server.uri(), "DEMO-1", "", false)
            .await
            .unwrap();
        assert_eq!(again.len(), 2);
    }

    #[tokio::test]
    async fn add_comment_with_mention_sends_adf() {
        let server = wiremock::MockServer::start().await;
        let mentions = vec![("@Alice".into(), "acc-1".into())];
        let body = crate::api::adf::comment_body_with_mentions("hi @Alice", &mentions);
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/rest/api/3/issue/DEMO-1/comment"))
            .and(wiremock::matchers::body_json(
                serde_json::json!({ "body": body }),
            ))
            .respond_with(wiremock::ResponseTemplate::new(201))
            .mount(&server)
            .await;

        let client = JiraClient::new("u@example.com", "token", false);
        client
            .add_comment(&server.uri(), "DEMO-1", "hi @Alice", &mentions)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn watch_issue_posts_watcher() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/rest/api/3/myself"))
            .respond_with(
                wiremock::ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "accountId": "me-1" })),
            )
            .mount(&server)
            .await;
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path(
                "/rest/api/3/issue/DEMO-1/watchers",
            ))
            .and(wiremock::matchers::body_json(
                serde_json::json!({ "accountId": "me-1" }),
            ))
            .respond_with(wiremock::ResponseTemplate::new(204))
            .mount(&server)
            .await;

        let client = JiraClient::new("u@example.com", "token", false);
        client.watch_issue(&server.uri(), "DEMO-1").await.unwrap();
    }

    #[tokio::test]
    async fn update_due_date_sends_put() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("PUT"))
            .and(wiremock::matchers::path("/rest/api/3/issue/DEMO-1"))
            .and(wiremock::matchers::body_json(serde_json::json!({
                "fields": { "duedate": "2026-12-31" }
            })))
            .respond_with(wiremock::ResponseTemplate::new(204))
            .mount(&server)
            .await;

        let client = JiraClient::new("u@example.com", "token", false);
        let d = chrono::NaiveDate::from_ymd_opt(2026, 12, 31).unwrap();
        client
            .update_due_date(&server.uri(), "DEMO-1", Some(d))
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

    #[tokio::test]
    async fn assign_to_account_sends_put() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("PUT"))
            .and(wiremock::matchers::path("/rest/api/3/issue/DEMO-1"))
            .and(wiremock::matchers::body_json(serde_json::json!({
                "fields": { "assignee": { "accountId": "acc-99" } }
            })))
            .respond_with(wiremock::ResponseTemplate::new(204))
            .mount(&server)
            .await;

        let client = JiraClient::new("u@example.com", "token", false);
        client
            .assign_to_account(&server.uri(), "DEMO-1", "acc-99")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn unassign_clears_assignee() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("PUT"))
            .and(wiremock::matchers::path("/rest/api/3/issue/DEMO-1"))
            .and(wiremock::matchers::body_json(serde_json::json!({
                "fields": { "assignee": null }
            })))
            .respond_with(wiremock::ResponseTemplate::new(204))
            .mount(&server)
            .await;

        let client = JiraClient::new("u@example.com", "token", false);
        client.unassign(&server.uri(), "DEMO-1").await.unwrap();
    }

    #[tokio::test]
    async fn get_workflow_transitions_parses_target_status() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path(
                "/rest/api/3/issue/DEMO-1/transitions",
            ))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "transitions": [
                        { "id": "21", "name": "Done", "to": { "name": "Done" } },
                        { "id": "31", "name": "Start Progress", "to": { "name": "In Progress" } }
                    ]
                })),
            )
            .mount(&server)
            .await;

        let client = JiraClient::new("u@example.com", "token", false);
        let options = client
            .get_workflow_transitions(&server.uri(), "DEMO-1")
            .await
            .unwrap();
        assert_eq!(options.len(), 2);
        assert_eq!(options[0].to_status, "Done");
        assert_eq!(options[1].label(), "Start Progress → In Progress");
    }

    #[tokio::test]
    async fn transition_issue_posts_transition_id() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path(
                "/rest/api/3/issue/DEMO-1/transitions",
            ))
            .and(wiremock::matchers::body_json(
                serde_json::json!({ "transition": { "id": "21" } }),
            ))
            .respond_with(wiremock::ResponseTemplate::new(204))
            .mount(&server)
            .await;

        let client = JiraClient::new("u@example.com", "token", false);
        let tr = types::WorkflowTransition {
            id: "21".into(),
            name: "Done".into(),
            to_status: "Done".into(),
            required_fields: vec![],
        };
        client
            .transition_issue(&server.uri(), "DEMO-1", &tr, &HashMap::new())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn get_workflow_transitions_parses_required_fields() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path(
                "/rest/api/3/issue/DEMO-1/transitions",
            ))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "transitions": [{
                        "id": "21",
                        "name": "Done",
                        "to": { "name": "Done" },
                        "fields": {
                            "resolution": {
                                "required": true,
                                "name": "Resolution",
                                "schema": { "type": "resolution" },
                                "allowedValues": [
                                    { "id": "10000", "name": "Done" }
                                ]
                            }
                        }
                    }]
                })),
            )
            .mount(&server)
            .await;

        let client = JiraClient::new("u@example.com", "token", false);
        let options = client
            .get_workflow_transitions(&server.uri(), "DEMO-1")
            .await
            .unwrap();
        assert_eq!(options[0].required_fields.len(), 1);
        assert_eq!(options[0].required_fields[0].id, "resolution");
    }

    #[tokio::test]
    async fn transition_issue_posts_required_fields() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path(
                "/rest/api/3/issue/DEMO-1/transitions",
            ))
            .and(wiremock::matchers::body_json(serde_json::json!({
                "transition": { "id": "21" },
                "fields": { "resolution": { "name": "Done" } }
            })))
            .respond_with(wiremock::ResponseTemplate::new(204))
            .mount(&server)
            .await;

        let client = JiraClient::new("u@example.com", "token", false);
        let tr = types::WorkflowTransition {
            id: "21".into(),
            name: "Done".into(),
            to_status: "Done".into(),
            required_fields: vec![],
        };
        let mut fields = HashMap::new();
        fields.insert("resolution".into(), serde_json::json!({ "name": "Done" }));
        client
            .transition_issue(&server.uri(), "DEMO-1", &tr, &fields)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn workflow_transition_reports_validation_error() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path(
                "/rest/api/3/issue/DEMO-1/transitions",
            ))
            .respond_with(
                wiremock::ResponseTemplate::new(400).set_body_json(serde_json::json!({
                    "errorMessages": ["Resolution is required"],
                    "errors": {}
                })),
            )
            .mount(&server)
            .await;

        let client = JiraClient::new("u@example.com", "token", false);
        let tr = types::WorkflowTransition {
            id: "21".into(),
            name: "Done".into(),
            to_status: "Done".into(),
            required_fields: vec![],
        };
        let err = client
            .transition_issue(&server.uri(), "DEMO-1", &tr, &HashMap::new())
            .await
            .unwrap_err();
        let errs =
            jira_error::field_errors(r#"{"errorMessages":["Resolution is required"],"errors":{}}"#);
        assert_eq!(errs.len(), 1);
        assert!(err.message.contains("Resolution is required"));
    }

    #[tokio::test]
    async fn workflow_transition_field_errors_parsed() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path(
                "/rest/api/3/issue/DEMO-1/transitions",
            ))
            .respond_with(
                wiremock::ResponseTemplate::new(400).set_body_json(serde_json::json!({
                    "errorMessages": [],
                    "errors": { "resolution": "Resolution is required" }
                })),
            )
            .mount(&server)
            .await;

        let client = JiraClient::new("u@example.com", "token", false);
        let tr = types::WorkflowTransition {
            id: "21".into(),
            name: "Done".into(),
            to_status: "Done".into(),
            required_fields: vec![],
        };
        let err = client
            .transition_issue(&server.uri(), "DEMO-1", &tr, &HashMap::new())
            .await
            .unwrap_err();
        assert_eq!(err.field_errors.len(), 1);
        assert_eq!(err.field_errors[0].0, "resolution");
    }

    #[tokio::test]
    async fn add_worklog_sends_time_spent() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/rest/api/3/issue/DEMO-1/worklog"))
            .and(wiremock::matchers::body_json(
                serde_json::json!({ "timeSpent": "30m" }),
            ))
            .respond_with(wiremock::ResponseTemplate::new(201))
            .mount(&server)
            .await;

        let client = JiraClient::new("u@example.com", "token", false);
        client
            .add_worklog(&server.uri(), "DEMO-1", "30m")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn issue_exists_on_success_and_missing() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/rest/api/3/issue/FOUND-1"))
            .respond_with(
                wiremock::ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "key": "FOUND-1" })),
            )
            .mount(&server)
            .await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/rest/api/3/issue/MISSING-1"))
            .respond_with(wiremock::ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let client = JiraClient::new("u@example.com", "token", false);
        assert!(client.issue_exists(&server.uri(), "FOUND-1").await);
        assert!(!client.issue_exists(&server.uri(), "MISSING-1").await);
    }

    #[tokio::test]
    async fn current_user_account_id_reads_myself() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/rest/api/3/myself"))
            .respond_with(
                wiremock::ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "accountId": "me-42" })),
            )
            .mount(&server)
            .await;

        let client = JiraClient::new("u@example.com", "token", false);
        let id = client.current_user_account_id(&server.uri()).await.unwrap();
        assert_eq!(id, "me-42");
        let cached = client.current_user_account_id(&server.uri()).await.unwrap();
        assert_eq!(cached, "me-42");
    }

    #[tokio::test]
    async fn create_issue_posts_fields_and_returns_key() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/rest/api/3/issue"))
            .respond_with(
                wiremock::ResponseTemplate::new(201)
                    .set_body_json(serde_json::json!({ "key": "DEMO-99", "id": "10099" })),
            )
            .mount(&server)
            .await;

        let client = JiraClient::new("u@example.com", "token", false);
        let fields = serde_json::json!({
            "project": { "key": "DEMO" },
            "issuetype": { "name": "Task" },
            "summary": "New from tick"
        });
        let key = client
            .create_issue(&server.uri(), &fields)
            .await
            .expect("create");
        assert_eq!(key, "DEMO-99");
    }

    #[tokio::test]
    async fn create_issue_reports_field_errors() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/rest/api/3/issue"))
            .respond_with(
                wiremock::ResponseTemplate::new(400).set_body_json(serde_json::json!({
                    "errorMessages": [],
                    "errors": { "components": "Components is required." }
                })),
            )
            .mount(&server)
            .await;

        let client = JiraClient::new("u@example.com", "token", false);
        let fields = serde_json::json!({
            "project": { "key": "DEMO" },
            "issuetype": { "name": "Task" },
            "summary": "X"
        });
        let err = client
            .create_issue(&server.uri(), &fields)
            .await
            .expect_err("validation");
        assert!(!err.field_errors.is_empty());
        assert_eq!(err.field_errors[0].0, "components");
    }
}
