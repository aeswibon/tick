use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

use crate::app::{App, InputMode};

mod detail_actions;
pub(crate) use detail_actions::parse_due_date_input;
mod mentions;
mod normal;
pub mod text;
pub use text::{buffer_for_submit, insert_paste, multiline_input_mode};
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

pub fn handle_paste(app: &mut App, pasted: String) {
    if app.input_mode == InputMode::None && !app.filtering {
        return;
    }
    if app.filtering {
        text::insert_paste(&mut app.filter, &pasted);
        app.invalidate_filter_cache();
        return;
    }
    text::insert_paste_at(&mut app.input_buffer, &mut app.input_cursor, &pasted);
    if app.input_mode == InputMode::GlobalSearchQuery {
        app.global_search_hits = crate::global_search::refresh_hits(app, &app.input_buffer);
        app.global_search_selected = 0;
    }
}

pub fn submit_comment_attach_path(app: &mut App) {
    let path = std::path::PathBuf::from(app.input_buffer.trim());
    app.reset_input_buffer();
    app.input_mode = InputMode::Comment;
    if path.as_os_str().is_empty() {
        app.status
            .set_action_error("Attachment path cannot be empty");
        return;
    }
    if !path.is_file() {
        app.status
            .set_action_error(format!("Not a file: {}", path.display()));
        return;
    }
    let name = path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string());
    app.comment_attach_paths.push(path);
    app.status.set_action_notice(format!(
        "Queued attachment {name} ({} pending)",
        app.comment_attach_paths.len()
    ));
}

fn clipboard_image_path() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join("tick-clipboard");
    let _ = std::fs::create_dir_all(&dir);
    let name = format!(
        "paste-{}-{}.png",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
    );
    dir.join(name)
}

async fn refresh_footer_input_side_effects(app: &mut App) {
    if app.input_mode == InputMode::GlobalSearchQuery {
        app.global_search_hits = crate::global_search::refresh_hits(app, &app.input_buffer);
        app.global_search_selected = 0;
    } else if mentions_enabled(app.input_mode)
        && mentions::active_mention_query(&app.input_buffer).is_some()
    {
        refresh_mention_picker(app).await;
    } else if mentions_enabled(app.input_mode) {
        clear_mention_picker(app);
    } else if app.input_mode == InputMode::TransitionField && app.transition_field_user_search {
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

async fn paste_clipboard_text(app: &mut App) {
    let Some(text) = crate::platform::read_from_clipboard() else {
        app.status
            .set_action_error("Clipboard empty or unavailable");
        return;
    };
    text::insert_paste_at(&mut app.input_buffer, &mut app.input_cursor, &text);
    refresh_footer_input_side_effects(app).await;
}

async fn paste_clipboard_image_attachment(app: &mut App) {
    let path = clipboard_image_path();
    if crate::platform::save_clipboard_image(&path) {
        let name = path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string());
        app.comment_attach_paths.push(path);
        app.status.set_action_notice(format!(
            "Queued clipboard image {name} ({} pending)",
            app.comment_attach_paths.len()
        ));
        return;
    }
    paste_clipboard_text(app).await;
}

/// Poll and dispatch one terminal event. Returns `true` to quit.
pub async fn handle_event(app: &mut App, event: Event) -> bool {
    match event {
        Event::Key(key) if key.kind == crossterm::event::KeyEventKind::Press => {
            handle_key(app, key).await
        }
        Event::Paste(text) => {
            handle_paste(app, text);
            false
        }
        _ => false,
    }
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

    if app
        .create_session
        .as_ref()
        .is_some_and(|s| s.step == crate::create_flow::CreateStep::DuplicateReview)
        && app.input_mode != InputMode::DuplicateFieldEdit
    {
        crate::create_flow::handle_duplicate_review_key(app, code);
        return false;
    }

    if app.showing_global_search && app.input_mode == InputMode::GlobalSearchQuery {
        if matches!(
            code,
            KeyCode::Esc
                | KeyCode::Enter
                | KeyCode::Up
                | KeyCode::Down
                | KeyCode::Char('j')
                | KeyCode::Char('k')
        ) {
            handle_global_search_key(app, code).await;
            return false;
        }
    } else if app.showing_global_search {
        handle_global_search_key(app, code).await;
        return false;
    }

    if app.showing_create_picker {
        crate::create_flow::handle_create_picker_key(app, code).await;
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

        if matches!(code, KeyCode::Char('p') | KeyCode::Char('P'))
            && key.modifiers.contains(KeyModifiers::CONTROL)
            && app.input_mode == InputMode::Comment
        {
            crate::ui::comment_preview::toggle_comment_preview(app);
            return false;
        }

        if matches!(code, KeyCode::Char('u') | KeyCode::Char('U'))
            && key.modifiers.contains(KeyModifiers::CONTROL)
            && app.input_mode == InputMode::Comment
        {
            app.input_mode = InputMode::CommentAttachPath;
            app.reset_input_buffer();
            app.comment_preview = false;
            clear_mention_picker(app);
            return false;
        }

        if crate::ui::comment_preview::comment_preview_active(app) {
            match code {
                KeyCode::Esc => {
                    app.comment_preview = false;
                    return false;
                }
                KeyCode::Enter => {
                    submit_input(app).await;
                    return false;
                }
                _ => return false,
            }
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
            KeyCode::Char('v') | KeyCode::Char('V')
                if key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                if key.modifiers.contains(KeyModifiers::SHIFT)
                    && app.input_mode == InputMode::Comment
                {
                    paste_clipboard_image_attachment(app).await;
                } else {
                    paste_clipboard_text(app).await;
                }
            }
            KeyCode::Char('a') | KeyCode::Char('A')
                if key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                app.input_cursor = 0;
            }
            KeyCode::Char('e') | KeyCode::Char('E')
                if key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                app.input_cursor = app.input_buffer.len();
            }
            KeyCode::Left if key.modifiers.contains(KeyModifiers::ALT) => {
                app.input_cursor = text::cursor_word_left(&app.input_buffer, app.input_cursor);
            }
            KeyCode::Right if key.modifiers.contains(KeyModifiers::ALT) => {
                app.input_cursor = text::cursor_word_right(&app.input_buffer, app.input_cursor);
            }
            KeyCode::Left => {
                app.input_cursor = text::cursor_left(&app.input_buffer, app.input_cursor);
            }
            KeyCode::Right => {
                app.input_cursor = text::cursor_right(&app.input_buffer, app.input_cursor);
            }
            KeyCode::Home => {
                app.input_cursor = text::cursor_home(&app.input_buffer, app.input_cursor);
            }
            KeyCode::End => {
                app.input_cursor = text::cursor_end(&app.input_buffer, app.input_cursor);
            }
            KeyCode::Char(c) => {
                text::insert_char(&mut app.input_buffer, &mut app.input_cursor, c);
                refresh_footer_input_side_effects(app).await;
            }
            KeyCode::Backspace if key.modifiers.contains(KeyModifiers::CONTROL) => {
                text::delete_word_backward(&mut app.input_buffer, &mut app.input_cursor);
                refresh_footer_input_side_effects(app).await;
            }
            KeyCode::Backspace => {
                text::backspace_at(&mut app.input_buffer, &mut app.input_cursor);
                refresh_footer_input_side_effects(app).await;
            }
            KeyCode::Delete => {
                text::delete_forward(&mut app.input_buffer, &mut app.input_cursor);
                refresh_footer_input_side_effects(app).await;
            }
            KeyCode::Esc => {
                clear_mention_picker(app);
                if app.input_mode == InputMode::CommentAttachPath {
                    app.input_mode = InputMode::Comment;
                    app.reset_input_buffer();
                    return false;
                }
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
                } else if app.input_mode == InputMode::DuplicateFieldEdit {
                    crate::create_flow::cancel_duplicate_field_edit(app);
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
                    app.comment_attach_paths.clear();
                    app.comment_preview = false;
                }
            }
            KeyCode::Enter if text::should_insert_newline_on_enter(&key) => {
                text::insert_char(&mut app.input_buffer, &mut app.input_cursor, '\n');
            }
            KeyCode::Enter => {
                if app.input_mode == InputMode::CommentAttachPath {
                    submit_comment_attach_path(app);
                    return false;
                }
                if app.input_mode == InputMode::OpenTicket {
                    submit_open_ticket(app).await;
                } else if app.input_mode == InputMode::DuplicateFieldEdit {
                    crate::create_flow::submit_duplicate_field_edit(app);
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
        crate::editable_fields::handle_editable_field_picker_key(app, code).await;
        return false;
    }

    if app.showing_custom_field_select {
        crate::editable_fields::handle_custom_field_select_key(app, code).await;
        return false;
    }

    if app.showing_custom_field_multi {
        crate::editable_fields::handle_custom_field_multi_key(app, code).await;
        return false;
    }

    if app.showing_sprints {
        handle_sprint_key(app, code).await;
        return false;
    }

    if app.input_mode == InputMode::None
        && !app.filtering
        && !app.show_help
        && app.try_plugin_key(&key)
    {
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
        } else if let Some(ticket) = app.selected_ticket_entry() {
            match app.toggle_bulk_mark(&ticket.site, &ticket.key) {
                Ok(true) => {
                    crate::hooks::fire_on_mark(&app.config, &ticket);
                }
                Ok(false) => {}
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
