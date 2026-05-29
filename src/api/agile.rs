use super::JiraClient;

/// Sprint picker entries: `("backlog", "Backlog")` or `(sprint_id, display name)`.
impl JiraClient {
    pub async fn list_sprint_targets(
        &self,
        base_url: &str,
        project_key: &str,
    ) -> Result<Vec<(String, String)>, String> {
        let mut options = vec![("backlog".into(), "Backlog (no sprint)".into())];
        let board_id = self.find_board_id(base_url, project_key).await?;
        let sprints = self.list_board_sprints(base_url, board_id).await?;
        for (id, name, state) in sprints {
            options.push((id, format!("{name} ({state})")));
        }
        Ok(options)
    }

    pub async fn move_issue_to_sprint_target(
        &self,
        base_url: &str,
        issue_key: &str,
        target_id: &str,
    ) -> Result<(), String> {
        if target_id == "backlog" {
            return self
                .move_issues_to_backlog(base_url, &[issue_key.to_string()])
                .await;
        }
        self.move_issues_to_sprint(base_url, target_id, &[issue_key.to_string()])
            .await
    }

    async fn find_board_id(&self, base_url: &str, project_key: &str) -> Result<u64, String> {
        let base = base_url.trim_end_matches('/');
        let url = format!("{base}/rest/agile/1.0/board?projectKeyOrId={project_key}");
        let resp = self
            .http
            .get(&url)
            .basic_auth(&self.email, Some(&self.token))
            .send()
            .await
            .map_err(|e| format!("HTTP error: {}", e))?;
        if !resp.status().is_success() {
            return Err(format!(
                "Board API {}: {}",
                resp.status(),
                resp.text().await.unwrap_or_default()
            ));
        }
        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Parse error: {}", e))?;
        let board_id = data["values"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|b| b["id"].as_u64())
            .ok_or_else(|| format!("No agile board found for project {project_key}"))?;
        Ok(board_id)
    }

    async fn list_board_sprints(
        &self,
        base_url: &str,
        board_id: u64,
    ) -> Result<Vec<(String, String, String)>, String> {
        let base = base_url.trim_end_matches('/');
        let url = format!("{base}/rest/agile/1.0/board/{board_id}/sprint?state=active,future");
        let resp = self
            .http
            .get(&url)
            .basic_auth(&self.email, Some(&self.token))
            .send()
            .await
            .map_err(|e| format!("HTTP error: {}", e))?;
        if !resp.status().is_success() {
            return Err(format!(
                "Sprint API {}: {}",
                resp.status(),
                resp.text().await.unwrap_or_default()
            ));
        }
        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Parse error: {}", e))?;
        let list = data["values"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|s| {
                        let id = s["id"].as_u64()?.to_string();
                        let name = s["name"].as_str()?.to_string();
                        let state = s["state"].as_str().unwrap_or("unknown").to_string();
                        Some((id, name, state))
                    })
                    .collect()
            })
            .unwrap_or_default();
        Ok(list)
    }

    async fn move_issues_to_sprint(
        &self,
        base_url: &str,
        sprint_id: &str,
        issue_keys: &[String],
    ) -> Result<(), String> {
        let url = format!(
            "{}/rest/agile/1.0/sprint/{sprint_id}/issue",
            base_url.trim_end_matches('/')
        );
        let resp = self
            .http
            .post(&url)
            .basic_auth(&self.email, Some(&self.token))
            .json(&serde_json::json!({ "issues": issue_keys }))
            .send()
            .await
            .map_err(|e| format!("HTTP error: {}", e))?;
        if resp.status().is_success() {
            Ok(())
        } else {
            Err(format!(
                "Move to sprint {}: {}",
                resp.status(),
                resp.text().await.unwrap_or_default()
            ))
        }
    }

    async fn move_issues_to_backlog(
        &self,
        base_url: &str,
        issue_keys: &[String],
    ) -> Result<(), String> {
        let url = format!(
            "{}/rest/agile/1.0/backlog/issue",
            base_url.trim_end_matches('/')
        );
        let resp = self
            .http
            .post(&url)
            .basic_auth(&self.email, Some(&self.token))
            .json(&serde_json::json!({ "issues": issue_keys }))
            .send()
            .await
            .map_err(|e| format!("HTTP error: {}", e))?;
        if resp.status().is_success() {
            Ok(())
        } else {
            Err(format!(
                "Move to backlog {}: {}",
                resp.status(),
                resp.text().await.unwrap_or_default()
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn list_sprint_targets_includes_backlog_and_sprints() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/rest/agile/1.0/board"))
            .and(wiremock::matchers::query_param("projectKeyOrId", "DEMO"))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "values": [{ "id": 7 }]
                })),
            )
            .mount(&server)
            .await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/rest/agile/1.0/board/7/sprint"))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "values": [
                        { "id": 42, "name": "Sprint 1", "state": "active" }
                    ]
                })),
            )
            .mount(&server)
            .await;

        let client = JiraClient::new("u@example.com", "token", false);
        let list = client
            .list_sprint_targets(&server.uri(), "DEMO")
            .await
            .unwrap();
        assert_eq!(list[0].0, "backlog");
        assert_eq!(list[1].0, "42");
        assert!(list[1].1.contains("Sprint 1"));
    }

    #[tokio::test]
    async fn move_to_sprint_posts_issue_keys() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/rest/agile/1.0/sprint/42/issue"))
            .and(wiremock::matchers::body_json(
                serde_json::json!({ "issues": ["DEMO-1"] }),
            ))
            .respond_with(wiremock::ResponseTemplate::new(204))
            .mount(&server)
            .await;

        let client = JiraClient::new("u@example.com", "token", false);
        client
            .move_issue_to_sprint_target(&server.uri(), "DEMO-1", "42")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn move_to_backlog_posts_issue_keys() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/rest/agile/1.0/backlog/issue"))
            .and(wiremock::matchers::body_json(
                serde_json::json!({ "issues": ["DEMO-1"] }),
            ))
            .respond_with(wiremock::ResponseTemplate::new(204))
            .mount(&server)
            .await;

        let client = JiraClient::new("u@example.com", "token", false);
        client
            .move_issue_to_sprint_target(&server.uri(), "DEMO-1", "backlog")
            .await
            .unwrap();
    }
}
