//! Issue links tab: add/remove links, jump, create subtask.

use crate::api::create::CreateError;
use crate::api::issue_relations::link_keys_for_picker;
use crate::api::types;
use crate::app::{App, InputMode};
use serde_json::json;

pub fn cancel_add_link(app: &mut App) {
    app.showing_add_link = false;
    if app.input_mode == InputMode::AddIssueLinkTarget {
        app.input_mode = InputMode::None;
        app.input_buffer.clear();
    }
}

pub fn start_add_link(app: &mut App) {
    if !app.detail_open {
        app.status
            .set_action_error("Open detail pane to add an issue link (I)");
        return;
    }
    if app.selected_ticket().is_none() {
        app.status.set_action_error("Select a ticket first");
        return;
    }
    app.showing_add_link = true;
    app.add_link_selected = 0;
}

pub async fn submit_add_link_target(app: &mut App) {
    let target = app.input_buffer.trim().to_uppercase();
    app.input_mode = InputMode::None;
    app.input_buffer.clear();
    app.showing_add_link = false;

    if target.is_empty() {
        app.status.set_action_error("Enter a target issue key");
        return;
    }

    let Some(sel) = app.selected_ticket() else {
        return;
    };
    let Some(base_url) = app.site_base_url(&sel.site) else {
        app.status.set_action_error("Unknown site for ticket");
        return;
    };

    let options = app.add_link_options();
    let idx = app.add_link_selected.min(options.len().saturating_sub(1));
    let (api_name, _label) = &options[idx];
    let (inward, outward) = link_keys_for_picker(idx, api_name, &sel.key, &target);

    app.loading = true;
    app.loading_message = Some(format!("Linking {} → {}…", sel.key, target));
    match app
        .jira
        .link_issues(&base_url, api_name, &inward, &outward)
        .await
    {
        Ok(()) => {
            app.status
                .set_action_notice(format!("Linked {} to {}", sel.key, target));
            app.issue_relations_key = None;
            app.refresh_issue_relations().await;
            app.refresh().await;
        }
        Err(e) => app.status.set_action_error(e),
    }
    app.loading = false;
    app.loading_message = None;
}

pub async fn remove_selected_link(app: &mut App) {
    if !app.links_selection_is_link_row() {
        app.status
            .set_action_error("Select an issue link row to remove (not a subtask)");
        return;
    }
    let Some(link_id) = app.selected_link_id() else {
        return;
    };
    let Some(sel) = app.selected_ticket() else {
        return;
    };
    let Some(base_url) = app.site_base_url(&sel.site) else {
        app.status.set_action_error("Unknown site for ticket");
        return;
    };

    app.loading = true;
    app.loading_message = Some("Removing link…".into());
    match app.jira.delete_issue_link(&base_url, &link_id).await {
        Ok(()) => {
            app.status.set_action_notice("Link removed");
            app.issue_relations_key = None;
            app.refresh_issue_relations().await;
            app.refresh().await;
        }
        Err(e) => app.status.set_action_error(e),
    }
    app.loading = false;
    app.loading_message = None;
}

pub fn start_create_subtask(app: &mut App) {
    if !app.detail_open {
        app.status
            .set_action_error("Open detail pane to create a subtask (Shift+N)");
        return;
    }
    if app.selected_ticket().is_none() {
        app.status.set_action_error("Select a parent issue first");
        return;
    }
    app.input_mode = InputMode::CreateSubtaskSummary;
    app.input_buffer.clear();
    app.status
        .set_action_notice("Subtask summary — Enter to create, Esc cancel");
}

pub async fn submit_create_subtask(app: &mut App) {
    let summary = app.input_buffer.trim().to_string();
    app.input_mode = InputMode::None;
    app.input_buffer.clear();

    if summary.is_empty() {
        app.status
            .set_action_error("Subtask summary cannot be empty");
        return;
    }

    let Some(parent) = app.selected_ticket() else {
        return;
    };
    let Some(base_url) = app.site_base_url(&parent.site) else {
        app.status.set_action_error("Unknown site for ticket");
        return;
    };

    let project_key = types::project_key_from_issue_key(&parent.key).to_string();

    app.loading = true;
    app.loading_message = Some("Loading subtask type…".into());
    let issue_types = match app
        .jira
        .list_issue_types_for_project(&base_url, &project_key)
        .await
    {
        Ok(t) => t,
        Err(e) => {
            app.loading = false;
            app.loading_message = None;
            app.status.set_action_error(e);
            return;
        }
    };

    let subtask_type = issue_types
        .iter()
        .find(|(_, name)| {
            let n = name.to_lowercase();
            n.contains("sub-task") || n.contains("subtask") || n == "sub task"
        })
        .map(|(_, name)| name.clone())
        .unwrap_or_else(|| "Sub-task".to_string());

    let fields = json!({
        "project": { "key": project_key },
        "issuetype": { "name": subtask_type },
        "summary": summary,
        "parent": { "key": parent.key },
    });

    app.loading_message = Some(format!("Creating subtask under {}…", parent.key));
    match app.jira.create_issue(&base_url, &fields).await {
        Ok(key) => {
            app.status
                .set_action_notice(format!("Created subtask {key} under {}", parent.key));
            app.issue_relations_key = None;
            app.refresh_issue_relations().await;
            app.refresh().await;
        }
        Err(CreateError { message, .. }) => app.status.set_action_error(message),
    }
    app.loading = false;
    app.loading_message = None;
}

pub async fn handle_add_link_key(app: &mut App, code: crossterm::event::KeyCode) {
    if !app.showing_add_link {
        return;
    }
    use crossterm::event::KeyCode;
    let count = app.add_link_options().len();
    match code {
        KeyCode::Esc => cancel_add_link(app),
        KeyCode::Up | KeyCode::Char('k') => {
            app.add_link_selected = app.add_link_selected.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.add_link_selected + 1 < count {
                app.add_link_selected += 1;
            }
        }
        KeyCode::Enter => {
            app.showing_add_link = false;
            app.input_mode = InputMode::AddIssueLinkTarget;
            app.input_buffer.clear();
            app.status
                .set_action_notice("Enter target issue key, then Enter");
        }
        KeyCode::Char(n) if ('1'..='4').contains(&n) => {
            let idx = (n as u8 - b'1') as usize;
            if idx < count {
                app.add_link_selected = idx;
            }
        }
        _ => {}
    }
}

pub async fn jump_to_selected_link(app: &mut App) {
    let Some(key) = app.selected_link_key() else {
        app.status
            .set_action_error("No link or subtask to jump to (j/k on Links tab)");
        return;
    };
    jump_to_issue_key(app, &key).await;
}

pub async fn open_selected_link_in_browser(app: &mut App) {
    let Some(key) = app.selected_link_key() else {
        app.status.set_action_error("No link or subtask selected");
        return;
    };
    app.loading = true;
    app.loading_message = Some(format!("Resolving {key}…"));
    match app.resolve_ticket_url(&key).await {
        Ok(url) => {
            if crate::platform::open_url(&url).is_err() {
                app.status.set_action_error("Could not open browser");
            }
        }
        Err(e) => app.status.set_action_error(e),
    }
    app.loading = false;
    app.loading_message = None;
}

async fn jump_to_issue_key(app: &mut App, key: &str) {
    if app.try_select_ticket_by_key(key) {
        if app.detail_tab == crate::app::DetailTab::Links {
            app.refresh_issue_relations().await;
        }
        app.status.set_action_notice(format!("Selected {key}"));
        return;
    }

    app.loading = true;
    app.loading_message = Some(format!("Looking up {key}…"));
    match app.resolve_ticket_url(key).await {
        Ok(url) => {
            if crate::platform::open_url(&url).is_ok() {
                app.status
                    .set_action_notice(format!("{key} not in current view — opened in browser"));
            } else {
                app.status.set_action_error("Could not open browser");
            }
        }
        Err(e) => app.status.set_action_error(e),
    }
    app.loading = false;
    app.loading_message = None;
}
