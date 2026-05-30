//! Multi-select table actions (transition, assign, watch).

use crate::api::types::WorkflowTransition;
use crate::app::App;
use crate::batch::{self, BatchOutcome};

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

    let keys: Vec<String> = refs.iter().map(|r| r.key.clone()).collect();
    let jira = app.jira.clone();
    let base = base_url.clone();
    let aid = account_id.clone();
    let outcome = run_bulk_with_progress(app, &keys, "Bulk assign", |i, total| {
        format!("Bulk assign {i}/{total}…")
    }, |key| {
        let jira = jira.clone();
        let base = base.clone();
        let aid = aid.clone();
        async move { jira.assign_to_account(&base, &key, &aid).await }
    })
    .await;
    app.refresh().await;
    bulk_result_notice(app, "Bulk assign", &outcome);
}

pub fn start_bulk_labels_edit(app: &mut App) {
    let refs = app.bulk_marked_refs_in_filter_order();
    if refs.is_empty() {
        return;
    }
    if app.bulk_same_site().is_none() {
        app.status
            .set_action_error("Bulk actions require a single site");
        return;
    }
    app.input_mode = crate::app::InputMode::BulkEditLabels;
    app.input_buffer.clear();
}

pub async fn submit_bulk_labels(app: &mut App) {
    let refs = app.bulk_marked_refs_in_filter_order();
    if refs.is_empty() {
        return;
    }
    let site = refs[0].site.clone();
    let Some(base_url) = app.site_base_url(&site) else {
        app.status.set_action_error("Unknown site for bulk labels");
        return;
    };
    let labels = crate::app::parse_labels_input(&app.input_buffer);
    let keys: Vec<String> = refs.iter().map(|r| r.key.clone()).collect();
    let jira = app.jira.clone();
    let base = base_url.clone();

    let outcome = run_bulk_with_progress(app, &keys, "Bulk labels", |i, total| {
        format!("Bulk labels {i}/{total}…")
    }, |key| {
        let jira = jira.clone();
        let base = base.clone();
        let labels = labels.clone();
        async move { jira.update_labels(&base, &key, &labels).await }
    })
    .await;
    app.input_mode = crate::app::InputMode::None;
    app.input_buffer.clear();
    app.refresh().await;
    bulk_result_notice(app, "Bulk labels", &outcome);
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
    let keys: Vec<String> = refs.iter().map(|r| r.key.clone()).collect();
    let jira = app.jira.clone();
    let base = base_url.clone();
    let outcome = run_bulk_with_progress(app, &keys, label, |i, total| {
        format!("{label} {i}/{total}…")
    }, |key| {
        let jira = jira.clone();
        let base = base.clone();
        async move {
            if unwatch {
                jira.unwatch_issue(&base, &key).await
            } else {
                jira.watch_issue(&base, &key).await
            }
        }
    })
    .await;
    app.refresh().await;
    bulk_result_notice(app, label, &outcome);
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
    let keys_owned: Vec<String> = keys.to_vec();
    let jira = app.jira.clone();
    let base = base_url.clone();
    let name = transition_name.clone();
    let outcome = run_bulk_with_progress(app, &keys_owned, "Bulk transition", |i, total| {
        format!("Bulk transition {i}/{total}…")
    }, |key| {
        let jira = jira.clone();
        let base = base.clone();
        let name = name.clone();
        async move {
            crate::operations::transition::apply_transition_by_name(&jira, &base, &key, &name).await
        }
    })
    .await;
    app.refresh().await;
    bulk_result_notice(app, "Bulk transition", &outcome);
}

async fn run_bulk_with_progress<F, Fut>(
    app: &mut App,
    keys: &[String],
    _label: &str,
    progress_msg: impl Fn(usize, usize) -> String,
    mut op: F,
) -> BatchOutcome
where
    F: FnMut(String) -> Fut,
    Fut: std::future::Future<Output = Result<(), String>>,
{
    let total = keys.len();
    app.loading = true;
    let mut outcome = BatchOutcome::default();
    for (i, key) in keys.iter().enumerate() {
        app.loading_message = Some(progress_msg(i + 1, total));
        let key = key.clone();
        match op(key.clone()).await {
            Ok(()) => outcome.ok += 1,
            Err(e) => outcome.failures.push(format!("{key}: {e}")),
        }
    }
    app.loading = false;
    app.loading_message = None;
    outcome
}

fn bulk_result_notice(app: &mut App, label: &str, outcome: &BatchOutcome) {
    app.status
        .set_action_notice(batch::format_batch_notice(label, outcome));
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
            hooks: Default::default(),
            view_jql: Config::build_view_jql(&Default::default()),
        };
        let jira = Arc::new(crate::api::JiraClient::new("a@b.com", "t", false));
        App::new(config, Theme::default(), jira, false)
    }
}
