use crate::app::App;

pub(crate) async fn assign_to_me(app: &mut App) {
    let Some(sel) = app.selected_ticket() else {
        return;
    };
    let Some(base_url) = app.site_base_url(&sel.site) else {
        app.status.set_action_error("Unknown site for ticket");
        return;
    };
    match app.jira.current_user_account_id(&base_url).await {
        Ok(account_id) => match app
            .jira
            .assign_to_account(&base_url, &sel.key, &account_id)
            .await
        {
            Ok(()) => app.refresh().await,
            Err(e) => app.status.set_action_error(e),
        },
        Err(e) => app.status.set_action_error(e),
    }
}

pub(crate) fn parse_due_date_input(buffer: &str) -> Result<Option<chrono::NaiveDate>, String> {
    let trimmed = buffer.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    chrono::NaiveDate::parse_from_str(trimmed, "%Y-%m-%d")
        .map(Some)
        .map_err(|_| "Due date must be YYYY-MM-DD (empty to clear)".into())
}

pub(crate) async fn watch_ticket(app: &mut App) {
    let Some(sel) = app.selected_ticket() else {
        app.status.set_action_error("Select a ticket first");
        return;
    };
    let Some(base_url) = app.site_base_url(&sel.site) else {
        app.status.set_action_error("Unknown site for ticket");
        return;
    };
    match app.jira.watch_issue(&base_url, &sel.key).await {
        Ok(()) => {
            app.status
                .set_action_notice(format!("Watching {}", sel.key));
            app.refresh().await;
        }
        Err(e) => app.status.set_action_error(e),
    }
}

pub(crate) async fn unwatch_ticket(app: &mut App) {
    let Some(sel) = app.selected_ticket() else {
        app.status.set_action_error("Select a ticket first");
        return;
    };
    let Some(base_url) = app.site_base_url(&sel.site) else {
        app.status.set_action_error("Unknown site for ticket");
        return;
    };
    match app.jira.unwatch_issue(&base_url, &sel.key).await {
        Ok(()) => {
            app.status
                .set_action_notice(format!("Unwatched {}", sel.key));
            app.refresh().await;
        }
        Err(e) => app.status.set_action_error(e),
    }
}

pub(crate) async fn unassign_ticket(app: &mut App) {
    let Some(sel) = app.selected_ticket() else {
        return;
    };
    let Some(base_url) = app.site_base_url(&sel.site) else {
        app.status.set_action_error("Unknown site for ticket");
        return;
    };
    match app.jira.unassign(&base_url, &sel.key).await {
        Ok(()) => app.refresh().await,
        Err(e) => app.status.set_action_error(e),
    }
}

pub(crate) async fn start_priority_picker(app: &mut App) {
    let Some(sel) = app.selected_ticket() else {
        return;
    };
    let Some(base_url) = app.site_base_url(&sel.site) else {
        return;
    };
    match app.jira.list_priorities(&base_url).await {
        Ok(options) if !options.is_empty() => {
            let current = app
                .selected_ticket_entry()
                .map(|t| t.priority)
                .unwrap_or_default();
            app.priority_options = options;
            app.priority_selected = app
                .priority_options
                .iter()
                .position(|(_, name)| name == &current)
                .unwrap_or(0);
            app.showing_priorities = true;
        }
        Ok(_) => app.status.set_action_error("No priorities available"),
        Err(e) => app.status.set_action_error(e),
    }
}

pub(crate) async fn start_sprint_picker(app: &mut App) {
    let Some(ticket) = app.selected_ticket_entry() else {
        return;
    };
    let Some(base_url) = app.site_base_url(&ticket.site) else {
        return;
    };
    let project = ticket.project_key_for_api().to_string();
    let board = app
        .config
        .sites
        .iter()
        .find(|s| s.name == ticket.site)
        .map(|s| s.board_config());
    match app
        .jira
        .list_sprint_targets(&base_url, &project, board.as_ref())
        .await
    {
        Ok(options) if options.len() > 1 => {
            let current = ticket.sprint_name.as_deref().unwrap_or("");
            app.sprint_options = options;
            app.sprint_selected = app
                .sprint_options
                .iter()
                .position(|(_, name)| name.contains(current) && !current.is_empty())
                .unwrap_or(0);
            app.showing_sprints = true;
        }
        Ok(_) => app
            .status
            .set_action_error("No sprints available for this project"),
        Err(e) => app.status.set_action_error(e),
    }
}

pub(crate) async fn start_status_picker(app: &mut App) {
    let Some(sel) = app.selected_ticket() else {
        app.status
            .set_action_error("Select a ticket to change status");
        return;
    };
    let Some(base_url) = app.site_base_url(&sel.site) else {
        app.status.set_action_error(format!(
            "Unknown site {:?} in config — cannot change status for {}",
            sel.site, sel.key
        ));
        return;
    };
    let current_status = app
        .selected_ticket_entry()
        .map(|t| t.status.clone())
        .unwrap_or_else(|| "Unknown".into());

    app.loading = true;
    app.loading_message = Some("Loading workflow transitions…".into());
    let (result, _) = tokio::join!(
        app.jira.get_workflow_transitions(&base_url, &sel.key),
        app.jira.warm_site_field_catalogs(&base_url)
    );
    app.loading = false;
    app.loading_message = None;

    match result {
        Ok(options) if !options.is_empty() => {
            app.transition_options = options;
            app.transition_selected = 0;
            app.showing_transitions = true;
        }
        Ok(_) => {
            app.status.set_action_error(format!(
                "No workflow transitions for {} (status: \"{}\"). The issue may be closed, or your role cannot move it in this workflow.",
                sel.key, current_status
            ));
        }
        Err(e) => app.status.set_action_error(e),
    }
}
