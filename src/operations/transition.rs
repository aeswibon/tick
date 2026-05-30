use std::collections::HashMap;
use std::sync::Arc;

use crate::api::{self, transition_fields, types::WorkflowTransition, JiraClient};

#[derive(Debug, Clone, serde::Serialize)]
pub struct TransitionSummary {
    pub id: String,
    pub name: String,
    pub to_status: String,
}

pub async fn list_transitions(
    jira: &Arc<JiraClient>,
    base_url: &str,
    key: &str,
) -> Result<Vec<TransitionSummary>, String> {
    let options = jira
        .get_workflow_transitions(base_url, key)
        .await
        .map_err(|e| e.to_string())?;
    Ok(options
        .into_iter()
        .map(|t| TransitionSummary {
            id: t.id,
            name: t.name,
            to_status: t.to_status,
        })
        .collect())
}

pub async fn apply_transition_by_name(
    jira: &Arc<JiraClient>,
    base_url: &str,
    key: &str,
    transition_name: &str,
) -> Result<(), String> {
    let options = jira
        .get_workflow_transitions(base_url, key)
        .await
        .map_err(|e| e.to_string())?;
    let Some(transition) = options.into_iter().find(|t| t.name == transition_name) else {
        return Err(format!("no transition '{transition_name}'"));
    };
    apply_prepared_transition(jira, base_url, key, transition).await
}

pub async fn apply_transition_by_id(
    jira: &Arc<JiraClient>,
    base_url: &str,
    key: &str,
    transition_id: &str,
) -> Result<(), String> {
    let options = jira
        .get_workflow_transitions(base_url, key)
        .await
        .map_err(|e| e.to_string())?;
    let Some(transition) = options.into_iter().find(|t| t.id == transition_id) else {
        return Err(format!("no transition id '{transition_id}'"));
    };
    apply_prepared_transition(jira, base_url, key, transition).await
}

async fn apply_prepared_transition(
    jira: &Arc<JiraClient>,
    base_url: &str,
    key: &str,
    mut transition: WorkflowTransition,
) -> Result<(), String> {
    if transition_fields::transition_needs_detail_fetch(&transition) {
        if let Ok(detail) = jira
            .get_transition_detail(base_url, key, &transition.id)
            .await
        {
            transition = detail;
        }
    }

    if transition.required_fields.is_empty() {
        if let Some(res) = transition_fields::infer_resolution_if_done_transition(
            &transition.name,
            &transition.to_status,
        ) {
            transition.required_fields.push(res);
        }
    }

    let pk = crate::api::types::project_key_from_issue_key(key);
    api::enrich_transition_fields(jira, base_url, Some(pk), &mut transition).await;

    if !transition.required_fields.is_empty() {
        return Err("transition requires fields (use interactive t in the TUI)".into());
    }

    jira.transition_issue(base_url, key, &transition, &HashMap::new())
        .await
        .map_err(|e| e.message)
}
