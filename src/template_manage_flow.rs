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
    let Some(session) = app.template_manage.as_mut() else {
        return;
    };
    match session.step {
        TemplateManageStep::List => match code {
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
        },
        TemplateManageStep::Actions => match code {
            KeyCode::Esc => {
                session.step = TemplateManageStep::List;
            }
            KeyCode::Char('e') => {
                let name = session.editing_name.clone();
                if let Some(t) = app.config.create.templates.iter().find(|t| t.name == name) {
                    app.input_buffer = t.summary.clone();
                }
                app.input_mode = InputMode::TemplateEditSummary;
                session.step = TemplateManageStep::EditSummary;
            }
            KeyCode::Char('p') => {
                let name = session.editing_name.clone();
                if let Some(t) = app.config.create.templates.iter().find(|t| t.name == name) {
                    app.input_buffer = t.project.clone();
                }
                app.input_mode = InputMode::TemplateEditProject;
                session.step = TemplateManageStep::EditProject;
            }
            KeyCode::Char('i') => {
                let name = session.editing_name.clone();
                if let Some(t) = app.config.create.templates.iter().find(|t| t.name == name) {
                    app.input_buffer = t.issue_type.clone();
                }
                app.input_mode = InputMode::TemplateEditIssueType;
                session.step = TemplateManageStep::EditIssueType;
            }
            KeyCode::Char('d') => session.step = TemplateManageStep::ConfirmDelete,
            _ => {}
        },
        TemplateManageStep::ConfirmDelete => match code {
            KeyCode::Esc => session.step = TemplateManageStep::Actions,
            KeyCode::Enter => {
                let name = session.editing_name.clone();
                match remove_template(&mut app.config, &name) {
                    Ok(()) => {
                        app.status
                            .set_action_notice(format!("Deleted template '{name}'"));
                        cancel_template_manage(app);
                    }
                    Err(e) => app.status.set_action_error(e),
                }
            }
            _ => {}
        },
        TemplateManageStep::EditSummary
        | TemplateManageStep::EditProject
        | TemplateManageStep::EditIssueType => {}
    }
}

pub async fn submit_template_edit(app: &mut App) {
    let field = match app.input_mode {
        InputMode::TemplateEditSummary => TemplateEditField::Summary,
        InputMode::TemplateEditProject => TemplateEditField::Project,
        InputMode::TemplateEditIssueType => TemplateEditField::IssueType,
        _ => return,
    };
    let value = app.input_buffer.trim().to_string();
    if value.is_empty() {
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
