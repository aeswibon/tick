//! Multi-select table actions (transition, assign, watch).

use std::collections::HashMap;

use crate::api::transition_fields;
use crate::api::types::WorkflowTransition;
use crate::api::{self};
use crate::app::App;

pub const BULK_MAX_SELECTED: usize = 50;

#[derive(Debug, Clone)]
pub enum BulkAction {
    Transition { site: String, keys: Vec<String> },
}

pub fn bulk_table_active(app: &App) -> bool {
    !app.detail_open
        && app.create_session.is_none()
        && app.template_manage.is_none()
        && app.template_export.is_none()
        && !app.showing_create_picker
}

pub fn mark_all_filtered(app: &mut App) {
    if !bulk_table_active(app) {
        return;
    }
    let tickets = crate::ticket_lock::read_tickets(&app.tickets);
    for idx in app.filtered_indices() {
        if app.bulk_marked.len() >= BULK_MAX_SELECTED {
            app.status.set_action_error(format!(
                "Bulk selection limited to {BULK_MAX_SELECTED} issues"
            ));
            break;
        }
        let t = &tickets[idx];
        app.bulk_marked
            .insert((t.site.clone(), t.key.clone()));
    }
}

pub async fn start_bulk_status_picker(app: &mut App) {
    let refs = app.bulk_marked_refs_in_filter_order();
    if refs.is_empty() {
        return;
    }
    let Some(site) = app.bulk_same_site() else {
        app.status
            .set_action_error("Bulk actions require a single site");
        return;
    };
    let first = &refs[0];
    let Some(base_url) = app.site_base_url(&first.site) else {
        app.status.set_action_error(format!(
            "Unknown site {:?} in config — cannot change status",
            first.site
        ));
        return;
    };

    app.loading = true;
    app.loading_message = Some("Loading workflow transitions…".into());
    let (result, _) = tokio::join!(
        app.jira.get_workflow_transitions(&base_url, &first.key),
        app.jira.warm_site_field_catalogs(&base_url)
    );
    app.loading = false;
    app.loading_message = None;

    match result {
        Ok(options) if !options.is_empty() => {
            let keys: Vec<String> = refs.into_iter().map(|r| r.key).collect();
            app.bulk_action = Some(BulkAction::Transition { site, keys });
            app.transition_options = options;
            app.transition_selected = 0;
            app.showing_transitions = true;
        }
        Ok(_) => app.status.set_action_error(format!(
            "No workflow transitions for {} (bulk picker)",
            first.key
        )),
        Err(e) => app.status.set_action_error(e),
    }
}

pub async fn bulk_assign_to_me(app: &mut App) {
    let refs = app.bulk_marked_refs_in_filter_order();
    if refs.is_empty() {
        return;
    }
    if app.bulk_same_site().is_none() {
        app.status
            .set_action_error("Bulk actions require a single site");
        return;
    }
    let site = refs[0].site.clone();
    let Some(base_url) = app.site_base_url(&site) else {
        app.status.set_action_error("Unknown site for bulk assign");
        return;
    };

    app.loading = true;
    let account_id = match app.jira.current_user_account_id(&base_url).await {
        Ok(id) => id,
        Err(e) => {
            app.loading = false;
            app.status.set_action_error(e);
            return;
        }
    };

    let total = refs.len();
    let mut ok = 0usize;
    let mut failures: Vec<String> = Vec::new();
    for (i, r) in refs.iter().enumerate() {
        app.loading_message = Some(format!("Bulk assign {}/{}…", i + 1, total));
        match app
            .jira
            .assign_to_account(&base_url, &r.key, &account_id)
            .await
        {
            Ok(()) => ok += 1,
            Err(e) => failures.push(format!("{}: {e}", r.key)),
        }
    }
    app.loading = false;
    app.loading_message = None;
    app.refresh().await;
    bulk_result_notice(app, "Bulk assign", ok, failures);
}

pub async fn bulk_watch(app: &mut App) {
    bulk_watch_toggle(app, false).await;
}

pub async fn bulk_unwatch(app: &mut App) {
    bulk_watch_toggle(app, true).await;
}

async fn bulk_watch_toggle(app: &mut App, unwatch: bool) {
    let refs = app.bulk_marked_refs_in_filter_order();
    if refs.is_empty() {
        return;
    }
    if app.bulk_same_site().is_none() {
        app.status
            .set_action_error("Bulk actions require a single site");
        return;
    }
    let site = refs[0].site.clone();
    let Some(base_url) = app.site_base_url(&site) else {
        app.status.set_action_error("Unknown site for bulk watch");
        return;
    };

    let label = if unwatch {
        "Bulk unwatch"
    } else {
        "Bulk watch"
    };
    let total = refs.len();
    let mut ok = 0usize;
    let mut failures: Vec<String> = Vec::new();

    app.loading = true;
    for (i, r) in refs.iter().enumerate() {
        app.loading_message = Some(format!("{label} {}/{}…", i + 1, total));
        let result = if unwatch {
            app.jira.unwatch_issue(&base_url, &r.key).await
        } else {
            app.jira.watch_issue(&base_url, &r.key).await
        };
        match result {
            Ok(()) => ok += 1,
            Err(e) => failures.push(format!("{}: {e}", r.key)),
        }
    }
    app.loading = false;
    app.loading_message = None;
    app.refresh().await;
    bulk_result_notice(app, label, ok, failures);
}

pub async fn apply_bulk_transition_by_name(
    app: &mut App,
    site: &str,
    keys: &[String],
    chosen: &WorkflowTransition,
) {
    let Some(base_url) = app.site_base_url(site) else {
        app.status.set_action_error("Unknown site for bulk transition");
        return;
    };

    let transition_name = chosen.name.clone();
    let total = keys.len();
    let mut ok = 0usize;
    let mut failures: Vec<String> = Vec::new();

    app.loading = true;
    for (i, key) in keys.iter().enumerate() {
        app.loading_message = Some(format!("Bulk transition {}/{}…", i + 1, total));
        match apply_named_transition(app, &base_url, key, &transition_name).await {
            Ok(()) => ok += 1,
            Err(reason) => failures.push(format!("{key}: {reason}")),
        }
    }
    app.loading = false;
    app.loading_message = None;
    app.refresh().await;
    bulk_result_notice(app, "Bulk transition", ok, failures);
}

async fn apply_named_transition(
    app: &App,
    base_url: &str,
    key: &str,
    transition_name: &str,
) -> Result<(), String> {
    let options = app
        .jira
        .get_workflow_transitions(base_url, key)
        .await
        .map_err(|e| e.to_string())?;
    let Some(mut transition) = options.into_iter().find(|t| t.name == transition_name) else {
        return Err(format!("no transition '{transition_name}'"));
    };

    if transition_fields::transition_needs_detail_fetch(&transition) {
        if let Ok(detail) = app
            .jira
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
    api::enrich_transition_fields(&app.jira, base_url, Some(pk), &mut transition).await;

    if !transition.required_fields.is_empty() {
        return Err("transition requires fields (use single-issue t)".into());
    }

    app.jira
        .transition_issue(base_url, key, &transition, &HashMap::new())
        .await
        .map_err(|e| e.message)
}

fn bulk_result_notice(app: &mut App, label: &str, ok: usize, failures: Vec<String>) {
    if failures.is_empty() {
        app.status
            .set_action_notice(format!("{label}: {ok} ok"));
        return;
    }
    let fail_summary = if failures.len() <= 2 {
        failures.join("; ")
    } else {
        format!("{}; …", failures[..2].join("; "))
    };
    app.status.set_action_notice(format!(
        "{label}: {ok} ok, {} failed ({fail_summary})",
        failures.len()
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;

    #[test]
    fn bulk_cap_enforced() {
        let mut app = test_app_with_bulk();
        for i in 0..BULK_MAX_SELECTED {
            assert!(app.toggle_bulk_mark("site", &format!("KEY-{i}")).is_ok());
        }
        assert_eq!(app.bulk_mark_count(), BULK_MAX_SELECTED);
        assert!(app.toggle_bulk_mark("site", "KEY-overflow").is_err());
        assert!(app.toggle_bulk_mark("site", "KEY-0").is_ok());
        assert_eq!(app.bulk_mark_count(), BULK_MAX_SELECTED - 1);
    }

    #[test]
    fn bulk_same_site_mixed_returns_none() {
        let mut app = test_app_with_bulk();
        app.bulk_marked.insert(("a".into(), "X-1".into()));
        app.bulk_marked.insert(("b".into(), "Y-1".into()));
        assert!(app.bulk_same_site().is_none());
        app.bulk_marked.clear();
        app.bulk_marked.insert(("zeta".into(), "Z-1".into()));
        app.bulk_marked.insert(("zeta".into(), "Z-2".into()));
        assert_eq!(app.bulk_same_site().as_deref(), Some("zeta"));
    }

    fn test_app_with_bulk() -> App {
        use crate::config::Config;
        use crate::theme::Theme;
        use std::sync::Arc;

        let config = Config {
            email: "a@b.com".into(),
            token: "t".into(),
            sites: vec![],
            columns: None,
            max_results: 50,
            page_size: 20,
            theme: "default".into(),
            views: Default::default(),
            notify_on_refresh: false,
            auth: Default::default(),
            oauth: Default::default(),
            create: Default::default(),
            view_jql: Config::build_view_jql(&Default::default()),
        };
        let jira = Arc::new(crate::api::JiraClient::new("a@b.com", "t", false));
        App::new(config, Theme::default(), jira, false)
    }
}
