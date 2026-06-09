//! List, edit, and delete issue templates from config (`Shift+E`).

use crate::app::{App, InputMode};
use crate::template_persist::{remove_template, update_template_field, TemplateEditField};
use crossterm::event::KeyCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateManageStep {
    List,
    Actions,
    EditSummary,
    EditProject,
    EditIssueType,
    EditDescription,
    EditLabels,
    ConfirmDelete,
}

pub struct TemplateManageSession {
    pub step: TemplateManageStep,
    pub names: Vec<String>,
    pub selected: usize,
    pub editing_name: String,
}

pub fn cancel_template_manage(app: &mut App) {
    app.template_manage = None;
    if matches!(
        app.input_mode,
        InputMode::TemplateEditSummary
            | InputMode::TemplateEditProject
            | InputMode::TemplateEditIssueType
            | InputMode::TemplateEditDescription
            | InputMode::TemplateEditLabels
    ) {
        app.input_mode = InputMode::None;
        app.input_buffer.clear();
    }
}

pub fn start_template_manage(app: &mut App) {
    if app.template_manage.is_some()
        || app.create_session.is_some()
        || app.template_export.is_some()
    {
        return;
    }
    let names: Vec<String> = app
        .config
        .create
        .templates
        .iter()
        .map(|t| t.name.clone())
        .collect();
    if names.is_empty() {
        app.status.set_action_error(
            "No templates — add [[create.templates]] or use X to export from a ticket",
        );
        return;
    }
    app.template_manage = Some(TemplateManageSession {
        step: TemplateManageStep::List,
        names,
        selected: 0,
        editing_name: String::new(),
    });
}

pub async fn handle_template_manage_key(app: &mut App, code: KeyCode) {
    let step = match app.template_manage.as_ref() {
        Some(session) => session.step,
        None => return,
    };
    match step {
        TemplateManageStep::List => {
            let Some(session) = app.template_manage.as_mut() else {
                return;
            };
            match code {
                KeyCode::Esc => cancel_template_manage(app),
                KeyCode::Char('j') | KeyCode::Down => {
                    if session.selected + 1 < session.names.len() {
                        session.selected += 1;
                    }
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    if session.selected > 0 {
                        session.selected -= 1;
                    }
                }
                KeyCode::Enter => {
                    session.editing_name = session.names[session.selected].clone();
                    session.step = TemplateManageStep::Actions;
                }
                _ => {}
            }
        }
        TemplateManageStep::Actions => handle_template_manage_actions(app, code),
        TemplateManageStep::ConfirmDelete => {
            let editing_name = app
                .template_manage
                .as_ref()
                .map(|s| s.editing_name.clone())
                .unwrap_or_default();
            match code {
                KeyCode::Esc => {
                    if let Some(session) = app.template_manage.as_mut() {
                        session.step = TemplateManageStep::Actions;
                    }
                }
                KeyCode::Enter => match remove_template(&mut app.config, &editing_name) {
                    Ok(()) => {
                        app.status
                            .set_action_notice(format!("Deleted template '{editing_name}'"));
                        cancel_template_manage(app);
                    }
                    Err(e) => app.status.set_action_error(e),
                },
                _ => {}
            }
        }
        TemplateManageStep::EditSummary
        | TemplateManageStep::EditProject
        | TemplateManageStep::EditIssueType
        | TemplateManageStep::EditDescription
        | TemplateManageStep::EditLabels => {}
    }
}

fn handle_template_manage_actions(app: &mut App, code: KeyCode) {
    let editing_name = app
        .template_manage
        .as_ref()
        .map(|s| s.editing_name.clone())
        .unwrap_or_default();
    match code {
        KeyCode::Esc => {
            if let Some(session) = app.template_manage.as_mut() {
                session.step = TemplateManageStep::List;
            }
        }
        KeyCode::Char('e') => {
            let footer = app
                .config
                .create
                .templates
                .iter()
                .find(|t| t.name == editing_name)
                .map(|t| t.summary.clone());
            if let Some(session) = app.template_manage.as_mut() {
                session.step = TemplateManageStep::EditSummary;
            }
            app.input_mode = InputMode::TemplateEditSummary;
            if let Some(text) = footer {
                app.set_footer_input(text);
            }
        }
        KeyCode::Char('p') => {
            let footer = app
                .config
                .create
                .templates
                .iter()
                .find(|t| t.name == editing_name)
                .map(|t| t.project.clone());
            if let Some(session) = app.template_manage.as_mut() {
                session.step = TemplateManageStep::EditProject;
            }
            app.input_mode = InputMode::TemplateEditProject;
            if let Some(text) = footer {
                app.set_footer_input(text);
            }
        }
        KeyCode::Char('i') => {
            let footer = app
                .config
                .create
                .templates
                .iter()
                .find(|t| t.name == editing_name)
                .map(|t| t.issue_type.clone());
            if let Some(session) = app.template_manage.as_mut() {
                session.step = TemplateManageStep::EditIssueType;
            }
            app.input_mode = InputMode::TemplateEditIssueType;
            if let Some(text) = footer {
                app.set_footer_input(text);
            }
        }
        KeyCode::Char('b') => {
            let footer = app
                .config
                .create
                .templates
                .iter()
                .find(|t| t.name == editing_name)
                .map(|t| t.description.clone());
            if let Some(session) = app.template_manage.as_mut() {
                session.step = TemplateManageStep::EditDescription;
            }
            app.input_mode = InputMode::TemplateEditDescription;
            if let Some(text) = footer {
                app.set_footer_input(text);
            }
        }
        KeyCode::Char('l') => {
            let footer = app
                .config
                .create
                .templates
                .iter()
                .find(|t| t.name == editing_name)
                .map(|t| t.labels.join(", "));
            if let Some(session) = app.template_manage.as_mut() {
                session.step = TemplateManageStep::EditLabels;
            }
            app.input_mode = InputMode::TemplateEditLabels;
            if let Some(text) = footer {
                app.set_footer_input(text);
            }
        }
        KeyCode::Char('d') => {
            if let Some(session) = app.template_manage.as_mut() {
                session.step = TemplateManageStep::ConfirmDelete;
            }
        }
        _ => {}
    }
}

pub async fn submit_template_edit(app: &mut App) {
    let field = match app.input_mode {
        InputMode::TemplateEditSummary => TemplateEditField::Summary,
        InputMode::TemplateEditProject => TemplateEditField::Project,
        InputMode::TemplateEditIssueType => TemplateEditField::IssueType,
        InputMode::TemplateEditDescription => TemplateEditField::Description,
        InputMode::TemplateEditLabels => TemplateEditField::Labels,
        _ => return,
    };
    let value = if matches!(
        app.input_mode,
        InputMode::TemplateEditDescription | InputMode::TemplateEditLabels
    ) {
        app.input_buffer.clone()
    } else {
        app.input_buffer.trim().to_string()
    };
    if value.is_empty()
        && !matches!(
            app.input_mode,
            InputMode::TemplateEditDescription | InputMode::TemplateEditLabels
        )
    {
        app.status.set_action_error("Value cannot be empty");
        return;
    }
    let Some(session) = app.template_manage.as_ref() else {
        return;
    };
    let name = session.editing_name.clone();
    match update_template_field(&mut app.config, &name, field, value) {
        Ok(()) => {
            app.status
                .set_action_notice(format!("Updated template '{name}'"));
            if let Some(session) = app.template_manage.as_mut() {
                session.step = TemplateManageStep::Actions;
                session.names = app
                    .config
                    .create
                    .templates
                    .iter()
                    .map(|t| t.name.clone())
                    .collect();
            }
            app.input_mode = InputMode::None;
            app.input_buffer.clear();
        }
        Err(e) => app.status.set_action_error(e),
    }
}
