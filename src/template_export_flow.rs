//! Interactive export of the selected issue as a saved create template.

use crossterm::event::KeyCode;

use crate::api::create::{enrich_draft_from_clone, seed_draft_from_ticket, CreateDraft};
use crate::app::{App, InputMode};
use crate::template_export::{
    append_issue_template, build_issue_template, exportable_field_rows,
    template_name_from_key_and_summary, TemplateFieldRow,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateExportStep {
    /// Space toggles which fields to store from the issue.
    IncludeFields,
    /// Space toggles which included fields should be left empty at create time.
    ClearValues,
    /// Footer prompt for template name.
    Name,
}

#[derive(Debug, Clone)]
pub struct TemplateExportSession {
    pub step: TemplateExportStep,
    pub selected: usize,
    pub rows: Vec<TemplateFieldRow>,
    pub draft: CreateDraft,
    pub source_key: String,
    pub site_name: String,
    pub default_name: String,
}

pub fn cancel_template_export(app: &mut App) {
    app.template_export = None;
    if app.input_mode == InputMode::TemplateExportName {
        app.input_mode = InputMode::None;
        app.input_buffer.clear();
    }
}

pub async fn start_template_export_from_selection(app: &mut App) {
    if app.template_export.is_some() || app.create_session.is_some() {
        return;
    }
    let Some(ticket) = app.selected_ticket_entry() else {
        app.status
            .set_action_error("Select a ticket to export as template (X)");
        return;
    };
    let Some(site) = app.config.sites.iter().find(|s| s.name == ticket.site) else {
        app.status
            .set_action_error(format!("Unknown site {:?} for ticket", ticket.site));
        return;
    };

    app.loading = true;
    app.loading_message = Some(format!("Loading {} for template export…", ticket.key));

    let mut draft = CreateDraft {
        base_url: site.base_url.clone(),
        site_name: site.name.clone(),
        ..Default::default()
    };
    seed_draft_from_ticket(&mut draft, &ticket, "");

    let sprint_field = site.sprint_field.as_deref();
    match app
        .jira
        .fetch_issue_for_clone(&site.base_url, &ticket.key, sprint_field)
        .await
    {
        Ok(issue) => {
            enrich_draft_from_clone(app.jira.as_ref(), &mut draft, &issue, sprint_field).await;
        }
        Err(e) => {
            app.loading = false;
            app.loading_message = None;
            app.status
                .set_action_error(format!("Could not load full issue: {e}"));
            return;
        }
    }

    app.loading = false;
    app.loading_message = None;

    let default_name = template_name_from_key_and_summary(&ticket.key, &draft.summary);
    let rows = exportable_field_rows(&draft, sprint_field);

    app.template_export = Some(TemplateExportSession {
        step: TemplateExportStep::IncludeFields,
        selected: 0,
        rows,
        draft,
        source_key: ticket.key,
        site_name: site.name.clone(),
        default_name,
    });
}

fn navigable_row_indices(session: &TemplateExportSession) -> Vec<usize> {
    match session.step {
        TemplateExportStep::IncludeFields => (0..session.rows.len()).collect(),
        TemplateExportStep::ClearValues => session
            .rows
            .iter()
            .enumerate()
            .filter(|(_, r)| r.include)
            .map(|(i, _)| i)
            .collect(),
        TemplateExportStep::Name => Vec::new(),
    }
}

pub async fn handle_template_export_key(app: &mut App, code: KeyCode) {
    let Some(session) = app.template_export.as_mut() else {
        return;
    };

    let nav = navigable_row_indices(session);
    if nav.is_empty() {
        cancel_template_export(app);
        return;
    }

    let pos_in_nav = nav.iter().position(|&i| i == session.selected).unwrap_or(0);

    match session.step {
        TemplateExportStep::IncludeFields | TemplateExportStep::ClearValues => match code {
            KeyCode::Esc => cancel_template_export(app),
            KeyCode::Up | KeyCode::Char('k') => {
                let new_pos = pos_in_nav.saturating_sub(1);
                session.selected = nav[new_pos];
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if pos_in_nav + 1 < nav.len() {
                    session.selected = nav[pos_in_nav + 1];
                }
            }
            KeyCode::Char(' ') => {
                let row = &mut session.rows[session.selected];
                if session.step == TemplateExportStep::IncludeFields {
                    row.include = !row.include;
                    if !row.include {
                        row.clear_value = false;
                    }
                } else {
                    row.clear_value = !row.clear_value;
                }
            }
            KeyCode::Enter => advance_template_export_step(app),
            KeyCode::Char(n) if ('1'..='9').contains(&n) => {
                let idx = (n as u8 - b'1') as usize;
                if idx < nav.len() {
                    session.selected = nav[idx];
                    let row = &mut session.rows[session.selected];
                    if session.step == TemplateExportStep::IncludeFields {
                        row.include = !row.include;
                        if !row.include {
                            row.clear_value = false;
                        }
                    } else {
                        row.clear_value = !row.clear_value;
                    }
                }
            }
            _ => {}
        },
        TemplateExportStep::Name => {}
    }
}

fn advance_template_export_step(app: &mut App) {
    let Some(session) = app.template_export.as_mut() else {
        return;
    };

    match session.step {
        TemplateExportStep::IncludeFields => {
            if !session.rows.iter().any(|r| r.include) {
                app.status
                    .set_action_error("Select at least one field to include (Space)");
                return;
            }
            session.step = TemplateExportStep::ClearValues;
            session.selected = session
                .rows
                .iter()
                .enumerate()
                .find(|(_, r)| r.include)
                .map(|(i, _)| i)
                .unwrap_or(0);
        }
        TemplateExportStep::ClearValues => {
            session.step = TemplateExportStep::Name;
            app.input_mode = InputMode::TemplateExportName;
            app.input_buffer = session.default_name.clone();
        }
        TemplateExportStep::Name => {}
    }
}

pub async fn submit_template_export_name(app: &mut App) {
    let name = app.input_buffer.trim().to_string();
    if name.is_empty() {
        app.status.set_action_error("Template name cannot be empty");
        return;
    }

    let Some(session) = app.template_export.take() else {
        return;
    };

    let sprint_field = app
        .config
        .sites
        .iter()
        .find(|s| s.name == session.site_name)
        .and_then(|s| s.sprint_field.as_deref());

    let template = build_issue_template(
        &name,
        &session.site_name,
        &session.draft,
        &session.rows,
        sprint_field,
    );

    if let Err(e) = template.validate_fields() {
        app.template_export = Some(session);
        app.input_mode = InputMode::TemplateExportName;
        app.input_buffer = name;
        app.status.set_action_error(e);
        return;
    }

    match append_issue_template(&app.config, &template, &session.source_key) {
        Ok(path) => {
            app.config.create.templates.push(template);
            app.status.set_action_notice(format!(
                "Template '{name}' saved to {} — press N to use",
                path.display()
            ));
        }
        Err(e) => {
            app.template_export = Some(session);
            app.input_mode = InputMode::TemplateExportName;
            app.input_buffer = name;
            app.status.set_action_error(e);
        }
    }

    app.input_mode = InputMode::None;
    app.input_buffer.clear();
}
