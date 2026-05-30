//! Add issue link wizard (`I` in detail).

use crate::api::issue_relations::{link_keys_for_picker, ADD_LINK_TYPES};
use crate::app::{App, InputMode};

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

    let idx = app
        .add_link_selected
        .min(ADD_LINK_TYPES.len().saturating_sub(1));
    let (api_name, _label) = ADD_LINK_TYPES[idx];
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
            app.refresh_all().await;
        }
        Err(e) => app.status.set_action_error(e),
    }
    app.loading = false;
    app.loading_message = None;
}

pub async fn handle_add_link_key(app: &mut App, code: crossterm::event::KeyCode) {
    if !app.showing_add_link {
        return;
    }
    use crossterm::event::KeyCode;
    match code {
        KeyCode::Esc => cancel_add_link(app),
        KeyCode::Up | KeyCode::Char('k') => {
            app.add_link_selected = app.add_link_selected.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.add_link_selected + 1 < ADD_LINK_TYPES.len() {
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
            if idx < ADD_LINK_TYPES.len() {
                app.add_link_selected = idx;
            }
        }
        _ => {}
    }
}
