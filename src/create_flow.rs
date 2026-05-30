//! Create and duplicate issue wizard.

use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent};

use crate::api::create::{
    apply_template_to_draft, build_create_fields, enrich_draft_from_clone, seed_draft_from_ticket,
    template_picker_label, CreateDraft,
};
use crate::api::transition_fields::{self, TransitionField, TransitionFieldKind};
use crate::api::{self, types::WorkflowTransition};
use crate::app::{App, InputMode};
use crate::config::IssueTemplate;
use crate::config::Site;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreateStep {
    Site,
    Project,
    IssueType,
    Template,
    Summary,
    Description,
}

#[derive(Debug, Clone)]
pub struct CreateSession {
    pub draft: CreateDraft,
    pub step: CreateStep,
    pub picker_options: Vec<(String, String)>,
    pub picker_selected: usize,
    pub required_pending: Vec<TransitionField>,
    pub required_values: HashMap<String, serde_json::Value>,
    /// Reuse transition-field modal for required create fields.
    pub showing_required_field: bool,
    /// Live markdown preview while editing description (`Ctrl+P`).
    pub description_preview: bool,
}

pub fn cancel_create(app: &mut App) {
    app.create_session = None;
    app.showing_create_picker = false;
    app.showing_transition_field = false;
    app.transition_field_text_mode = false;
    app.transition_multi_mode = false;
    app.transition_multi_picked.clear();
    app.transition_field_user_search = false;
    app.transition_field_current = None;
    app.transition_field_options.clear();
    if app.input_mode == InputMode::CreateField || app.input_mode == InputMode::CreateDescription {
        app.input_mode = InputMode::None;
        app.input_buffer.clear();
        app.input_mentions.clear();
    }
}

fn site_by_name<'a>(sites: &'a [Site], name: &str) -> Option<&'a Site> {
    sites.iter().find(|s| s.name == name)
}

fn clone_prefix(app: &App) -> String {
    app.config.create.clone_summary_prefix.clone()
}

pub async fn start_create_from_template(app: &mut App) {
    cancel_create(app);
    let site_filter = if app.config.sites.len() == 1 {
        Some(app.config.sites[0].name.as_str())
    } else {
        None
    };
    let templates: Vec<&IssueTemplate> = app.config.issue_templates_for_site(site_filter);
    if templates.is_empty() {
        app.status.set_action_error(
            "No issue templates in config — add [[create.templates]] (see tick --init comments)",
        );
        return;
    }
    let picker_options: Vec<(String, String)> = templates
        .iter()
        .map(|t| (t.name.clone(), template_picker_label(t)))
        .collect();
    app.create_session = Some(CreateSession {
        draft: CreateDraft::default(),
        step: CreateStep::Template,
        picker_options,
        picker_selected: 0,
        required_pending: Vec::new(),
        required_values: HashMap::new(),
        showing_required_field: false,
        description_preview: false,
    });
    app.showing_create_picker = true;
}

pub async fn start_create_blank(app: &mut App) {
    cancel_create(app);
    let mut draft = CreateDraft::default();
    if app.config.sites.len() == 1 {
        let site = &app.config.sites[0];
        draft.site_name = site.name.clone();
        draft.base_url = site.base_url.clone();
        apply_site_defaults(&mut draft, site);
        app.create_session = Some(CreateSession {
            draft,
            step: next_step_after_site(app, &site.name),
            picker_options: Vec::new(),
            picker_selected: 0,
            required_pending: Vec::new(),
            required_values: HashMap::new(),
            showing_required_field: false,
            description_preview: false,
        });
        advance_create_step(app).await;
    } else {
        let options: Vec<(String, String)> = app
            .config
            .sites
            .iter()
            .map(|s| (s.name.clone(), format!("{} — {}", s.name, s.base_url)))
            .collect();
        app.create_session = Some(CreateSession {
            draft,
            step: CreateStep::Site,
            picker_options: options,
            picker_selected: 0,
            required_pending: Vec::new(),
            required_values: HashMap::new(),
            showing_required_field: false,
            description_preview: false,
        });
        app.showing_create_picker = true;
    }
}

pub async fn start_create_duplicate(app: &mut App) {
    let Some(ticket) = app.selected_ticket_entry() else {
        app.status
            .set_action_error("Select a ticket to duplicate (C)");
        return;
    };
    let Some(site) = site_by_name(&app.config.sites, &ticket.site) else {
        app.status
            .set_action_error(format!("Unknown site {:?} for ticket", ticket.site));
        return;
    };
    let site_name = site.name.clone();
    let base_url = site.base_url.clone();
    let sprint_field = site.sprint_field.clone();
    let ticket_key = ticket.key.clone();

    cancel_create(app);
    let mut draft = CreateDraft {
        site_name,
        base_url: base_url.clone(),
        ..Default::default()
    };
    seed_draft_from_ticket(&mut draft, &ticket, &clone_prefix(app));

    app.loading = true;
    app.loading_message = Some("Loading issue fields for duplicate…".into());
    match app
        .jira
        .fetch_issue_for_clone(&draft.base_url, &ticket_key, sprint_field.as_deref())
        .await
    {
        Ok(issue) => {
            enrich_draft_from_clone(&app.jira, &mut draft, &issue, sprint_field.as_deref()).await;
        }
        Err(e) => {
            app.status
                .set_action_error(format!("Could not load full issue (using list data): {e}"));
        }
    }
    app.loading = false;
    app.loading_message = None;

    app.create_session = Some(CreateSession {
        draft,
        step: CreateStep::Summary,
        picker_options: Vec::new(),
        picker_selected: 0,
        required_pending: Vec::new(),
        required_values: HashMap::new(),
        showing_required_field: false,
        description_preview: false,
    });
    begin_summary_input(app);
}

fn apply_site_defaults(draft: &mut CreateDraft, site: &Site) {
    if let Some(ref p) = site.create_project {
        draft.project_key = p.clone();
    }
    if let Some(ref t) = site.create_issue_type {
        draft.issue_type_name = t.clone();
    }
}

fn next_step_after_site(app: &App, site_name: &str) -> CreateStep {
    let Some(site) = site_by_name(&app.config.sites, site_name) else {
        return CreateStep::Project;
    };
    if site.create_project.is_none() {
        return CreateStep::Project;
    }
    if site.create_issue_type.is_none() {
        return CreateStep::IssueType;
    }
    CreateStep::Summary
}

fn assignable_context_key(draft: &CreateDraft) -> String {
    draft
        .source_key
        .clone()
        .unwrap_or_else(|| format!("{}.__CREATE__", draft.project_key))
}

async fn advance_create_step(app: &mut App) {
    let step = app.create_session.as_ref().map(|s| s.step);
    let project_key = app
        .create_session
        .as_ref()
        .map(|s| s.draft.project_key.clone())
        .unwrap_or_default();
    let issue_type = app
        .create_session
        .as_ref()
        .map(|s| s.draft.issue_type_name.clone())
        .unwrap_or_default();

    match step {
        Some(CreateStep::Site) => {}
        Some(CreateStep::Project) => {
            if project_key.is_empty() {
                load_project_picker(app).await;
                return;
            }
            if let Some(session) = app.create_session.as_mut() {
                session.step = CreateStep::IssueType;
            }
            if issue_type.is_empty() {
                load_issue_type_picker(app).await;
                return;
            }
            if let Some(session) = app.create_session.as_mut() {
                session.step = CreateStep::Summary;
            }
            begin_summary_input(app);
        }
        Some(CreateStep::IssueType) => {
            if issue_type.is_empty() {
                load_issue_type_picker(app).await;
                return;
            }
            if let Some(session) = app.create_session.as_mut() {
                session.step = CreateStep::Summary;
            }
            begin_summary_input(app);
        }
        Some(CreateStep::Summary) | Some(CreateStep::Description) | Some(CreateStep::Template) => {}
        None => {}
    }
}

async fn load_project_picker(app: &mut App) {
    let base_url = app
        .create_session
        .as_ref()
        .map(|s| s.draft.base_url.clone());
    let Some(base_url) = base_url else {
        return;
    };
    app.loading = true;
    app.loading_message = Some("Loading projects…".into());
    match app.jira.search_projects(&base_url).await {
        Ok(options) if !options.is_empty() => {
            if let Some(session) = app.create_session.as_mut() {
                session.picker_options = options;
                session.picker_selected = 0;
                session.step = CreateStep::Project;
            }
            app.showing_create_picker = true;
        }
        Ok(_) => app
            .status
            .set_action_error("No projects found for your account"),
        Err(e) => app.status.set_action_error(e),
    }
    app.loading = false;
    app.loading_message = None;
}

async fn load_issue_type_picker(app: &mut App) {
    let (base_url, project_key) = app
        .create_session
        .as_ref()
        .map(|s| (s.draft.base_url.clone(), s.draft.project_key.clone()))
        .unwrap_or_default();
    if project_key.is_empty() {
        app.status.set_action_error("Choose a project first (p)");
        return;
    }
    app.loading = true;
    app.loading_message = Some("Loading issue types…".into());
    match app
        .jira
        .list_issue_types_for_project(&base_url, &project_key)
        .await
    {
        Ok(options) if !options.is_empty() => {
            if let Some(session) = app.create_session.as_mut() {
                session.picker_options = options;
                session.picker_selected = 0;
                session.step = CreateStep::IssueType;
            }
            app.showing_create_picker = true;
        }
        Ok(_) => app
            .status
            .set_action_error("No issue types for this project"),
        Err(e) => app.status.set_action_error(e),
    }
    app.loading = false;
    app.loading_message = None;
}

fn begin_summary_input(app: &mut App) {
    app.showing_create_picker = false;
    if let Some(session) = app.create_session.as_mut() {
        session.step = CreateStep::Summary;
        app.input_buffer = session.draft.summary.clone();
    }
    app.input_mode = InputMode::CreateField;
}

fn begin_description_input(app: &mut App) {
    if let Some(session) = app.create_session.as_mut() {
        session.step = CreateStep::Description;
        session.description_preview = false;
        app.input_buffer = session.draft.description.clone();
        app.input_mentions.clear();
    }
    app.input_mode = InputMode::CreateDescription;
}

pub fn toggle_create_description_preview(app: &mut App) {
    if app.input_mode != InputMode::CreateDescription {
        return;
    }
    if let Some(session) = app.create_session.as_mut() {
        session.description_preview = !session.description_preview;
        if session.description_preview {
            app.showing_mention_picker = false;
        }
    }
}

pub fn create_description_preview_active(app: &App) -> bool {
    app.input_mode == InputMode::CreateDescription
        && app
            .create_session
            .as_ref()
            .is_some_and(|s| s.description_preview)
}

async fn load_required_and_prompt(app: &mut App) {
    let (base_url, project_key, issue_type) = app
        .create_session
        .as_ref()
        .map(|s| {
            (
                s.draft.base_url.clone(),
                s.draft.project_key.clone(),
                s.draft.issue_type_name.clone(),
            )
        })
        .unwrap_or_default();
    app.loading = true;
    app.loading_message = Some("Checking required fields…".into());
    let mut pending = match app
        .jira
        .required_fields_for_create(&base_url, &project_key, &issue_type)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            app.status.set_action_error(e);
            Vec::new()
        }
    };
    if !pending.is_empty() {
        let mut wt = WorkflowTransition {
            id: String::new(),
            name: String::new(),
            to_status: String::new(),
            required_fields: pending,
        };
        api::enrich_transition_fields(&app.jira, &base_url, Some(&project_key), &mut wt).await;
        pending = wt.required_fields;
    }
    app.loading = false;
    app.loading_message = None;

    if let Some(session) = app.create_session.as_mut() {
        session.required_pending = pending;
    }
    if !begin_next_create_required(app) {
        submit_create(app).await;
    }
}

fn begin_next_create_required(app: &mut App) -> bool {
    let Some(session) = app.create_session.as_mut() else {
        return false;
    };
    if session.required_pending.is_empty() {
        return false;
    }
    let field = session.required_pending[0].clone();
    let remaining = session.required_pending.len();
    app.transition_field_current = Some(field.clone());
    app.transition_field_heading = if remaining > 1 {
        format!("Create: {} ({} more)", field.name, remaining - 1)
    } else {
        format!("Create: {}", field.name)
    };
    app.showing_transition_field = true;
    app.showing_create_picker = false;
    session.showing_required_field = true;

    match field.kind {
        TransitionFieldKind::User => {
            app.transition_field_text_mode = true;
            app.transition_field_user_search = true;
            app.transition_field_options.clear();
            app.input_mode = InputMode::CreateField;
            app.input_buffer.clear();
        }
        TransitionFieldKind::Picker | TransitionFieldKind::Boolean if !field.options.is_empty() => {
            app.transition_multi_mode = false;
            app.transition_multi_picked.clear();
            app.transition_field_text_mode = false;
            app.transition_field_options = field.options;
            app.transition_field_selected = 0;
        }
        TransitionFieldKind::MultiPicker if !field.options.is_empty() => {
            app.transition_multi_mode = true;
            app.transition_multi_picked = vec![false; field.options.len()];
            app.transition_field_text_mode = false;
            app.transition_field_options = field.options;
            app.transition_field_selected = 0;
        }
        _ => {
            app.transition_multi_mode = false;
            app.transition_multi_picked.clear();
            app.transition_field_text_mode = true;
            app.transition_field_options.clear();
            app.input_mode = InputMode::CreateField;
            app.input_buffer.clear();
        }
    }
    true
}

pub fn create_assignable_key(app: &App) -> Option<String> {
    app.create_session
        .as_ref()
        .map(|s| assignable_context_key(&s.draft))
}

pub async fn refresh_create_user_search(app: &mut App, force_refresh: bool) {
    if !app.transition_field_user_search {
        return;
    }
    let Some(session) = app.create_session.as_ref() else {
        return;
    };
    let base_url = session.draft.base_url.clone();
    let context_key = assignable_context_key(&session.draft);
    let query = app.input_buffer.trim();
    if force_refresh {
        app.loading = true;
        app.loading_message = Some("Loading more users…".into());
    }
    let api_query = if force_refresh { query } else { "" };
    let users = match app
        .jira
        .ensure_assignable_users(&base_url, &context_key, api_query, force_refresh)
        .await
    {
        Ok(catalog) => crate::api::assignable_users::filter_users(&catalog, query),
        Err(e) => {
            app.status.set_action_error(e);
            Vec::new()
        }
    };
    if force_refresh {
        app.loading = false;
        app.loading_message = None;
    }
    app.transition_field_options = users;
    app.transition_field_selected = app
        .transition_field_selected
        .min(app.transition_field_options.len().saturating_sub(1));
    app.showing_transition_field = true;
    app.transition_field_text_mode = false;
}

pub fn advance_create_required(app: &mut App, field: &TransitionField, value: serde_json::Value) {
    if let Some(session) = app.create_session.as_mut() {
        session.required_values.insert(field.id.clone(), value);
        if session.required_pending.first().map(|f| f.id.as_str()) == Some(field.id.as_str()) {
            session.required_pending.remove(0);
        } else {
            session.required_pending.retain(|f| f.id != field.id);
        }
    }
    app.showing_transition_field = false;
    app.transition_field_current = None;
    app.transition_field_user_search = false;
    if app.input_mode == InputMode::CreateField {
        app.input_mode = InputMode::None;
        app.input_buffer.clear();
    }
}

pub async fn apply_create_required_pick(app: &mut App, idx: usize) {
    if idx >= app.transition_field_options.len() {
        return;
    }
    let Some(field) = app.transition_field_current.clone() else {
        return;
    };
    let (id, label) = app.transition_field_options[idx].clone();
    let value = field.value_from_choice(&id, &label);
    advance_create_required(app, &field, value);
    if !begin_next_create_required(app) {
        submit_create(app).await;
    }
}

pub async fn prompt_next_create_required(app: &mut App) {
    if !begin_next_create_required(app) {
        submit_create(app).await;
    } else if app.transition_field_user_search {
        refresh_create_user_search(app, false).await;
    }
}

pub async fn submit_create(app: &mut App) {
    let Some(session) = app.create_session.take() else {
        return;
    };
    let draft = session.draft;
    let required_values = session.required_values;
    let source_key = draft.source_key.clone();
    let site_name = draft.site_name.clone();

    let fields = build_create_fields(&draft, &required_values);
    app.loading = true;
    app.loading_message = Some("Creating issue…".into());
    match app.jira.create_issue(&draft.base_url, &fields).await {
        Ok(new_key) => {
            if let Some(ref src) = source_key {
                if let Some(site) = site_by_name(&app.config.sites, &site_name) {
                    if site.clone_link_enabled() {
                        let link_type = site.clone_link_type_name();
                        if let Err(e) = app
                            .jira
                            .link_issues_clones(&draft.base_url, &new_key, src, link_type)
                            .await
                        {
                            app.status.set_action_error(format!(
                                "Created {new_key}, but clone link failed: {e}"
                            ));
                        }
                    }
                }
            }
            app.loading = false;
            app.loading_message = None;
            app.refresh().await;
            app.select_ticket_by_key(&new_key);
            app.detail_open = true;
            app.ensure_selected_issue_detail().await;
            app.status.clear_action_error();
        }
        Err(e) if !e.field_errors.is_empty() => {
            app.loading = false;
            app.loading_message = None;
            let pending: Vec<TransitionField> = e
                .field_errors
                .iter()
                .map(|(id, _)| transition_fields::field_for_error_key(id, &[]))
                .collect();
            app.create_session = Some(CreateSession {
                draft,
                step: CreateStep::Summary,
                picker_options: Vec::new(),
                picker_selected: 0,
                required_pending: pending,
                required_values,
                showing_required_field: false,
                description_preview: false,
            });
            app.status.set_action_error(e.message);
            if !begin_next_create_required(app) {
                app.create_session = None;
            } else if app.transition_field_user_search {
                refresh_create_user_search(app, false).await;
            }
        }
        Err(e) => {
            app.loading = false;
            app.loading_message = None;
            app.status.set_action_error(e.message);
        }
    }
}

pub async fn handle_create_picker_key(app: &mut App, code: KeyCode) {
    let count = app
        .create_session
        .as_ref()
        .map(|s| s.picker_options.len())
        .unwrap_or(0);
    match code {
        KeyCode::Esc => cancel_create(app),
        KeyCode::Up | KeyCode::Char('k') if count > 0 => {
            if let Some(session) = app.create_session.as_mut() {
                session.picker_selected = session.picker_selected.saturating_sub(1);
            }
        }
        KeyCode::Down | KeyCode::Char('j') if count > 0 => {
            if let Some(session) = app.create_session.as_mut() {
                if session.picker_selected + 1 < count {
                    session.picker_selected += 1;
                }
            }
        }
        KeyCode::Enter if count > 0 => {
            apply_create_picker_pick(app).await;
        }
        KeyCode::Char(n) if count > 0 && ('1'..='9').contains(&n) => {
            let idx = (n as u8 - b'1') as usize;
            if let Some(session) = app.create_session.as_mut() {
                session.picker_selected = idx.min(count.saturating_sub(1));
            }
            apply_create_picker_pick(app).await;
        }
        _ => {}
    }
}

async fn apply_create_picker_pick(app: &mut App) {
    let (step, selected, options) = {
        let Some(session) = app.create_session.as_mut() else {
            return;
        };
        (
            session.step,
            session.picker_selected,
            session.picker_options.clone(),
        )
    };
    let Some((id, _label)) = options.get(selected).cloned() else {
        return;
    };
    match step {
        CreateStep::Site => {
            if let Some(site) = site_by_name(&app.config.sites, &id) {
                let next = next_step_after_site(app, &id);
                if let Some(session) = app.create_session.as_mut() {
                    session.draft.site_name = site.name.clone();
                    session.draft.base_url = site.base_url.clone();
                    apply_site_defaults(&mut session.draft, site);
                    session.step = next;
                }
                app.showing_create_picker = false;
                advance_create_step(app).await;
            }
        }
        CreateStep::Project => {
            if let Some(session) = app.create_session.as_mut() {
                session.draft.project_key = id;
            }
            app.showing_create_picker = false;
            advance_create_step(app).await;
        }
        CreateStep::IssueType => {
            if let Some(session) = app.create_session.as_mut() {
                session.draft.issue_type_name = id;
            }
            app.showing_create_picker = false;
            advance_create_step(app).await;
        }
        CreateStep::Template => {
            apply_template_pick(app, &id).await;
        }
        _ => {}
    }
}

async fn apply_template_pick(app: &mut App, template_name: &str) {
    let Some(template) = app
        .config
        .create
        .templates
        .iter()
        .find(|t| t.name == template_name)
        .cloned()
    else {
        app.status
            .set_action_error(format!("Unknown template '{template_name}'"));
        return;
    };
    let site_name = template.site.clone().or_else(|| {
        if app.config.sites.len() == 1 {
            Some(app.config.sites[0].name.clone())
        } else {
            None
        }
    });
    let Some(site_name) = site_name else {
        app.status.set_action_error(format!(
            "Template '{}' needs site = \"...\" in config",
            template.name
        ));
        return;
    };
    let Some(site) = site_by_name(&app.config.sites, &site_name) else {
        app.status
            .set_action_error(format!("Unknown site '{site_name}' for template"));
        return;
    };

    if let Some(session) = app.create_session.as_mut() {
        apply_template_to_draft(&mut session.draft, &template, site);
        if !session.draft.priority_name.is_empty() {
            session.draft.priority_id = app
                .jira
                .resolve_priority_id(&session.draft.base_url, &session.draft.priority_name)
                .await;
        }
        session.step = CreateStep::Summary;
    }
    app.showing_create_picker = false;
    begin_summary_input(app);
}

pub async fn submit_create_input(app: &mut App) {
    let mode = app.input_mode;
    let buffer = app.input_buffer.trim().to_string();
    match mode {
        InputMode::CreateField if app.create_session.is_some() => {
            let step = app.create_session.as_ref().map(|s| s.step);
            if app
                .create_session
                .as_ref()
                .is_some_and(|s| s.showing_required_field)
            {
                if let Some(field) = app.transition_field_current.clone() {
                    if field.kind == TransitionFieldKind::User
                        && !app.transition_field_options.is_empty()
                    {
                        apply_create_required_pick(app, app.transition_field_selected).await;
                        return;
                    }
                    match field.value_from_text(&buffer) {
                        Ok(value) => {
                            advance_create_required(app, &field, value);
                            prompt_next_create_required(app).await;
                        }
                        Err(e) => app.status.set_action_error(e),
                    }
                }
                return;
            }
            if let Some(CreateStep::Summary) = step {
                if buffer.is_empty() {
                    app.status.set_action_error("Summary cannot be empty");
                    return;
                }
                if let Some(session) = app.create_session.as_mut() {
                    session.draft.summary = buffer;
                }
                app.input_buffer.clear();
                begin_description_input(app);
            }
        }
        InputMode::CreateDescription => {
            if let Some(session) = app.create_session.as_mut() {
                session.draft.description = buffer;
                session.draft.description_adf = None;
            }
            app.input_mode = InputMode::None;
            app.input_buffer.clear();
            load_required_and_prompt(app).await;
        }
        _ => {}
    }
}

pub async fn handle_create_field_key(app: &mut App, key: &KeyEvent) -> bool {
    if !app
        .create_session
        .as_ref()
        .is_some_and(|s| s.showing_required_field)
    {
        return false;
    }
    let code = key.code;
    let has_options = !app.transition_field_options.is_empty();
    match code {
        KeyCode::Esc => {
            cancel_create(app);
            true
        }
        _ if crate::input::load_more_users_key(key) => {
            refresh_create_user_search(app, true).await;
            true
        }
        KeyCode::Up | KeyCode::Char('k') if has_options => {
            app.transition_field_selected = app.transition_field_selected.saturating_sub(1);
            true
        }
        KeyCode::Down | KeyCode::Char('j') if has_options => {
            if app.transition_field_selected + 1 < app.transition_field_options.len() {
                app.transition_field_selected += 1;
            }
            true
        }
        KeyCode::Enter if has_options => {
            apply_create_required_pick(app, app.transition_field_selected).await;
            true
        }
        KeyCode::Char(n) if has_options && ('1'..='9').contains(&n) => {
            let idx = (n as u8 - b'1') as usize;
            apply_create_required_pick(app, idx).await;
            true
        }
        _ => false,
    }
}

pub async fn handle_create_normal_keys(app: &mut App, code: KeyCode) -> bool {
    if app.create_session.is_none() || app.input_mode != InputMode::None {
        return false;
    }
    match code {
        KeyCode::Char('p') if !app.showing_create_picker => {
            load_project_picker(app).await;
            true
        }
        KeyCode::Char('t') if !app.showing_create_picker => {
            load_issue_type_picker(app).await;
            true
        }
        _ => false,
    }
}
