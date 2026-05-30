use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{App, InputMode};

mod detail_actions;
mod mentions;
mod normal;
mod transitions;

#[cfg(test)]
mod key_tests;

use detail_actions::{unwatch_ticket, watch_ticket};
use mentions::{
    clear_mention_picker, handle_mention_picker_key, mentions_enabled, refresh_mention_picker,
    submit_input, submit_open_ticket,
};
use normal::handle_normal_key;
use transitions::{
    cancel_transition_collect, handle_priority_key, handle_sprint_key, handle_transition_field_key,
    handle_transition_key, handle_transition_multi_field_key, handle_transition_user_field_key,
    refresh_transition_user_search,
};

/// Shown in picker footers (⌘R on macOS; Ctrl+R elsewhere). Both work on macOS when the terminal reports modifiers.
#[cfg(target_os = "macos")]
pub const LOAD_MORE_USERS_KEYS_HINT: &str = "⌘R";
#[cfg(not(target_os = "macos"))]
pub const LOAD_MORE_USERS_KEYS_HINT: &str = "Ctrl+R";

#[cfg(target_os = "macos")]
pub const LOAD_MORE_USERS_USER_MODAL_HINT: &str =
    "Type in footer to filter; ⌘R fetch more users into cache; Enter to select";
#[cfg(not(target_os = "macos"))]
pub const LOAD_MORE_USERS_USER_MODAL_HINT: &str =
    "Type in footer to filter; Ctrl+R fetch more users into cache; Enter to select";

#[cfg(target_os = "macos")]
pub const LOAD_MORE_USERS_PICKER_FOOTER: &str = "  j/k move  Enter pick  ⌘R add users  Esc cancel";
#[cfg(not(target_os = "macos"))]
pub const LOAD_MORE_USERS_PICKER_FOOTER: &str =
    "  j/k move  Enter pick  Ctrl+R add users  Esc cancel";

#[cfg(target_os = "macos")]
pub const LOAD_MORE_USERS_FIELD_PICKER_FOOTER: &str =
    "  Type in footer to filter  j/k move  Enter pick  ⌘R add users  Esc cancel";
#[cfg(not(target_os = "macos"))]
pub const LOAD_MORE_USERS_FIELD_PICKER_FOOTER: &str =
    "  Type in footer to filter  j/k move  Enter pick  Ctrl+R add users  Esc cancel";

/// Load more assignable users from Jira (merge into cache). Plain `r`/`R` are for filtering.
pub fn load_more_users_key(key: &KeyEvent) -> bool {
    if !matches!(key.code, KeyCode::Char('r') | KeyCode::Char('R')) {
        return false;
    }
    let mods = key.modifiers;
    mods.contains(KeyModifiers::CONTROL)
        || mods.contains(KeyModifiers::SUPER)
        || mods.contains(KeyModifiers::META)
}

/// Returns `true` when the app should quit.
pub async fn handle_key(app: &mut App, key: KeyEvent) -> bool {
    let code = key.code;
    if app.showing_mention_picker {
        handle_mention_picker_key(app, &key).await;
        return false;
    }

    let create_required = app
        .create_session
        .as_ref()
        .is_some_and(|s| s.showing_required_field);
    if app.showing_add_link {
        crate::issue_relations_flow::handle_add_link_key(app, code).await;
        return false;
    }

    if app.showing_transition_field && app.transition_multi_mode {
        handle_transition_multi_field_key(app, code).await;
        return false;
    }

    if app.showing_transition_field && app.transition_field_user_search {
        if create_required {
            if crate::create_flow::handle_create_field_key(app, &key).await {
                return false;
            }
        } else if handle_transition_user_field_key(app, &key).await {
            return false;
        }
    } else if app.showing_transition_field && !app.transition_field_text_mode {
        if create_required {
            if crate::create_flow::handle_create_field_key(app, &key).await {
                return false;
            }
        } else {
            handle_transition_field_key(app, code).await;
        }
        return false;
    }

    if app.showing_create_picker {
        crate::create_flow::handle_create_picker_key(app, code).await;
        return false;
    }

    if app.showing_global_search {
        handle_global_search_key(app, code).await;
        return false;
    }

    if app.template_export.is_some() && app.input_mode != InputMode::TemplateExportName {
        crate::template_export_flow::handle_template_export_key(app, code).await;
        return false;
    }

    if app.template_manage.is_some()
        && !matches!(
            app.input_mode,
            InputMode::TemplateEditSummary
                | InputMode::TemplateEditProject
                | InputMode::TemplateEditIssueType
                | InputMode::TemplateEditDescription
                | InputMode::TemplateEditLabels
        )
    {
        crate::template_manage_flow::handle_template_manage_key(app, code).await;
        return false;
    }

    if app.filtering {
        match code {
            KeyCode::Char(c) => app.filter.push(c),
            KeyCode::Backspace => {
                app.filter.pop();
            }
            KeyCode::Esc | KeyCode::Enter => {
                app.filtering = false;
                app.go_to_first();
                app.invalidate_filter_cache();
            }
            _ => {}
        }
        return false;
    }

    if app.input_mode != InputMode::None {
        if matches!(code, KeyCode::Char('p') | KeyCode::Char('P'))
            && key.modifiers.contains(KeyModifiers::CONTROL)
            && app.input_mode == InputMode::CreateDescription
        {
            crate::create_flow::toggle_create_description_preview(app);
            return false;
        }

        if crate::create_flow::create_description_preview_active(app) {
            match code {
                KeyCode::Esc => {
                    if let Some(session) = app.create_session.as_mut() {
                        session.description_preview = false;
                    }
                    return false;
                }
                KeyCode::Enter => {
                    submit_input(app).await;
                    return false;
                }
                _ => return false,
            }
        }

        match code {
            KeyCode::Char(c) => {
                app.input_buffer.push(c);
                if app.input_mode == InputMode::GlobalSearchQuery {
                    app.global_search_hits =
                        crate::global_search::refresh_hits(app, &app.input_buffer);
                    app.global_search_selected = 0;
                } else if mentions_enabled(app.input_mode) {
                    refresh_mention_picker(app).await;
                } else if app.input_mode == InputMode::TransitionField
                    && app.transition_field_user_search
                {
                    refresh_transition_user_search(app, false).await;
                } else if app.input_mode == InputMode::CreateField
                    && app
                        .create_session
                        .as_ref()
                        .is_some_and(|s| s.showing_required_field)
                    && app.transition_field_user_search
                {
                    crate::create_flow::refresh_create_user_search(app, false).await;
                }
            }
            KeyCode::Backspace => {
                app.input_buffer.pop();
                if app.input_mode == InputMode::GlobalSearchQuery {
                    app.global_search_hits =
                        crate::global_search::refresh_hits(app, &app.input_buffer);
                    app.global_search_selected = 0;
                } else if mentions_enabled(app.input_mode) {
                    refresh_mention_picker(app).await;
                } else if app.input_mode == InputMode::TransitionField
                    && app.transition_field_user_search
                {
                    refresh_transition_user_search(app, false).await;
                } else if app.input_mode == InputMode::CreateField
                    && app
                        .create_session
                        .as_ref()
                        .is_some_and(|s| s.showing_required_field)
                    && app.transition_field_user_search
                {
                    crate::create_flow::refresh_create_user_search(app, false).await;
                }
            }
            KeyCode::Esc => {
                clear_mention_picker(app);
                if app.input_mode == InputMode::TransitionField {
                    cancel_transition_collect(app);
                } else if app.input_mode == InputMode::CreateDescription
                    && app
                        .create_session
                        .as_ref()
                        .is_some_and(|s| s.description_preview)
                {
                    if let Some(session) = app.create_session.as_mut() {
                        session.description_preview = false;
                    }
                } else if matches!(
                    app.input_mode,
                    InputMode::CreateField
                        | InputMode::CreateDescription
                        | InputMode::TemplateEditDescription
                        | InputMode::TemplateEditLabels
                ) {
                    if matches!(
                        app.input_mode,
                        InputMode::TemplateEditDescription | InputMode::TemplateEditLabels
                    ) {
                        if let Some(session) = app.template_manage.as_mut() {
                            session.step = crate::template_manage_flow::TemplateManageStep::Actions;
                        }
                        app.input_mode = InputMode::None;
                        app.input_buffer.clear();
                    } else {
                        crate::create_flow::cancel_create(app);
                    }
                } else if app.input_mode == InputMode::TemplateExportName {
                    crate::template_export_flow::cancel_template_export(app);
                } else if app.input_mode == InputMode::EditCustomField {
                    crate::editable_fields::cancel_custom_field_edit(app);
                } else if matches!(
                    app.input_mode,
                    InputMode::EditDueDate
                        | InputMode::ClosedSearchQuery
                        | InputMode::AddIssueLinkTarget
                        | InputMode::CreateSubtaskSummary
                        | InputMode::TemplateEditSummary
                        | InputMode::TemplateEditProject
                        | InputMode::TemplateEditIssueType
                ) {
                    if matches!(
                        app.input_mode,
                        InputMode::TemplateEditSummary
                            | InputMode::TemplateEditProject
                            | InputMode::TemplateEditIssueType
                    ) {
                        if let Some(session) = app.template_manage.as_mut() {
                            session.step = crate::template_manage_flow::TemplateManageStep::Actions;
                        }
                    }
                    app.input_mode = InputMode::None;
                    app.input_buffer.clear();
                } else {
                    app.input_mode = InputMode::None;
                    app.input_buffer.clear();
                    app.input_mentions.clear();
                }
            }
            KeyCode::Enter => {
                if app.input_mode == InputMode::OpenTicket {
                    submit_open_ticket(app).await;
                } else if matches!(
                    app.input_mode,
                    InputMode::CreateField | InputMode::CreateDescription
                ) {
                    crate::create_flow::submit_create_input(app).await;
                } else if app.input_mode == InputMode::TemplateExportName {
                    crate::template_export_flow::submit_template_export_name(app).await;
                } else if app.input_mode == InputMode::AddIssueLinkTarget {
                    crate::issue_relations_flow::submit_add_link_target(app).await;
                } else if app.input_mode == InputMode::CreateSubtaskSummary {
                    crate::issue_relations_flow::submit_create_subtask(app).await;
                } else if app.input_mode == InputMode::ClosedSearchQuery {
                    app.closed_search_query = app.input_buffer.trim().to_string();
                    app.input_mode = InputMode::None;
                    app.input_buffer.clear();
                    app.save_closed_prefs();
                    app.refresh_closed_search().await;
                } else if matches!(
                    app.input_mode,
                    InputMode::TemplateEditSummary
                        | InputMode::TemplateEditProject
                        | InputMode::TemplateEditIssueType
                        | InputMode::TemplateEditDescription
                        | InputMode::TemplateEditLabels
                ) {
                    crate::template_manage_flow::submit_template_edit(app).await;
                } else if app.input_mode == InputMode::BulkEditLabels {
                    crate::bulk::submit_bulk_labels(app).await;
                } else if app.input_mode == InputMode::GlobalSearchQuery {
                    if !app.global_search_hits.is_empty() {
                        let idx = app
                            .global_search_selected
                            .min(app.global_search_hits.len() - 1);
                        let hit = app.global_search_hits[idx].clone();
                        crate::global_search::jump_to_hit(app, &hit).await;
                    } else {
                        app.showing_global_search = false;
                        app.input_mode = InputMode::None;
                        app.input_buffer.clear();
                    }
                } else {
                    submit_input(app).await;
                }
            }
            _ => {}
        }
        return false;
    }

    if app.show_site_errors {
        handle_site_errors_key(app, code);
        return false;
    }

    if app.showing_transitions {
        handle_transition_key(app, code).await;
        return false;
    }

    if app.showing_priorities {
        handle_priority_key(app, code).await;
        return false;
    }

    if app.showing_editable_field_picker {
        crate::editable_fields::handle_editable_field_picker_key(app, code);
        return false;
    }

    if app.showing_custom_field_select {
        crate::editable_fields::handle_custom_field_select_key(app, code).await;
        return false;
    }

    if app.showing_sprints {
        handle_sprint_key(app, code).await;
        return false;
    }

    if code == KeyCode::Char('W') {
        if app.bulk_mark_count() > 0 && !app.detail_open {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                crate::bulk::bulk_unwatch(app).await;
            } else {
                crate::bulk::bulk_watch(app).await;
            }
        } else if key.modifiers.contains(KeyModifiers::SHIFT) {
            unwatch_ticket(app).await;
        } else {
            watch_ticket(app).await;
        }
        return false;
    }

    if matches!(code, KeyCode::Char('I') | KeyCode::Char('i')) {
        if key.modifiers.contains(KeyModifiers::SHIFT) {
            if app.detail_open && app.detail_tab == crate::app::DetailTab::Links {
                crate::issue_relations_flow::remove_selected_link(app).await;
            }
        } else if app.detail_open {
            crate::issue_relations_flow::start_add_link(app);
        }
        return false;
    }

    if matches!(code, KeyCode::Char('N') | KeyCode::Char('n'))
        && key.modifiers.contains(KeyModifiers::SHIFT)
        && app.detail_open
        && app.detail_tab == crate::app::DetailTab::Links
    {
        crate::issue_relations_flow::start_create_subtask(app);
        return false;
    }

    if matches!(code, KeyCode::Char('v') | KeyCode::Char('V')) && !app.detail_open {
        if key.modifiers.contains(KeyModifiers::SHIFT) {
            app.cycle_custom_view(false).await;
        } else {
            app.cycle_custom_view(true).await;
        }
        return false;
    }

    if matches!(code, KeyCode::Char('E'))
        && key.modifiers.contains(KeyModifiers::SHIFT)
        && !app.detail_open
    {
        crate::template_manage_flow::start_template_manage(app);
        return false;
    }

    if matches!(code, KeyCode::Char('g') | KeyCode::Char('G'))
        && key.modifiers.contains(KeyModifiers::CONTROL)
        && !app.detail_open
        && app.input_mode == InputMode::None
    {
        app.showing_global_search = true;
        app.input_mode = InputMode::GlobalSearchQuery;
        app.input_buffer.clear();
        app.global_search_hits = crate::global_search::refresh_hits(app, "");
        app.global_search_selected = 0;
        return false;
    }

    if matches!(code, KeyCode::Char(' ')) && crate::bulk::bulk_table_active(app) {
        if key.modifiers.contains(KeyModifiers::SHIFT) {
            crate::bulk::mark_all_filtered(app);
        } else if let Some(t) = app.selected_ticket_entry() {
            match app.toggle_bulk_mark(&t.site, &t.key) {
                Ok(()) => {}
                Err(e) => app.status.set_action_error(e),
            }
        }
        return false;
    }

    handle_normal_key(app, code).await
}

async fn handle_global_search_key(app: &mut App, code: KeyCode) {
    let hit_count = app.global_search_hits.len();
    match code {
        KeyCode::Esc => {
            app.showing_global_search = false;
            app.global_search_hits.clear();
            app.global_search_selected = 0;
            app.input_mode = InputMode::None;
            app.input_buffer.clear();
        }
        KeyCode::Enter if hit_count > 0 => {
            let idx = app.global_search_selected.min(hit_count - 1);
            let hit = app.global_search_hits[idx].clone();
            crate::global_search::jump_to_hit(app, &hit).await;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.global_search_selected = app.global_search_selected.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') if app.global_search_selected + 1 < hit_count => {
            app.global_search_selected += 1;
        }
        _ => {}
    }
}

fn handle_site_errors_key(app: &mut App, code: KeyCode) {
    let count = app.status.site_warnings.len();
    match code {
        KeyCode::Esc | KeyCode::Char('!') => {
            app.show_site_errors = false;
            app.site_error_scroll = 0;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.site_error_scroll = app.site_error_scroll.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') if app.site_error_scroll + 1 < count => {
            app.site_error_scroll += 1;
        }
        _ => {}
    }
}
