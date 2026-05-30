//! Export Jira issues as `[[create.templates]]` TOML and persist to config.

use crate::api::create::{template_picker_label, CreateDraft};
use crate::api::JiraClient;
use crate::config::{Config, IssueTemplate, Site};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(serde::Deserialize)]
struct TemplatesFile {
    #[serde(default)]
    templates: Vec<IssueTemplate>,
}

/// Load extra templates from `create.templates_file` (path relative to config dir unless absolute).
pub fn merge_templates_file(config: &mut Config) -> Result<(), String> {
    let Some(rel) = config.create.templates_file.clone() else {
        return Ok(());
    };
    let path = resolve_templates_path(&rel)?;
    if !path.is_file() {
        return Err(format!(
            "create.templates_file not found: {}",
            path.display()
        ));
    }
    let raw = std::fs::read_to_string(&path)
        .map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
    let extra: TemplatesFile =
        toml::from_str(&raw).map_err(|e| format!("Invalid templates file: {e}"))?;
    config.create.templates.extend(extra.templates);
    Ok(())
}

pub fn resolve_templates_path(rel: &str) -> Result<PathBuf, String> {
    let path = Path::new(rel);
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }
    let base = Config::config_dir()?;
    Ok(base.join(path))
}

/// Field ids the UI lets users include or clear when exporting a template.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TemplateFieldId {
    Summary,
    Description,
    Labels,
    Priority,
    Assignee,
    Parent,
    Sprint,
    DueDate,
}

#[derive(Debug, Clone)]
pub struct TemplateFieldRow {
    pub id: TemplateFieldId,
    pub label: String,
    pub preview: String,
    pub include: bool,
    pub clear_value: bool,
}

impl TemplateFieldRow {
    pub fn has_value(&self) -> bool {
        self.preview != "(empty)"
    }
}

/// Build picker rows from a loaded create draft.
pub fn exportable_field_rows(draft: &CreateDraft, sprint_field: Option<&str>) -> Vec<TemplateFieldRow> {
    let sprint_label = sprint_field
        .map(|id| format!("Sprint ({id})"))
        .unwrap_or_else(|| "Sprint".to_string());

    let rows = [
        (
            TemplateFieldId::Summary,
            "Summary",
            preview_line(&draft.summary, 72),
        ),
        (
            TemplateFieldId::Description,
            "Description",
            preview_line(&draft.description, 72),
        ),
        (
            TemplateFieldId::Labels,
            "Labels",
            if draft.labels.is_empty() {
                "(empty)".into()
            } else {
                draft.labels.join(", ")
            },
        ),
        (
            TemplateFieldId::Priority,
            "Priority",
            if draft.priority_name.is_empty() {
                "(empty)".into()
            } else {
                draft.priority_name.clone()
            },
        ),
        (
            TemplateFieldId::Assignee,
            "Assignee",
            draft
                .assignee_account_id
                .as_deref()
                .map(|id| format!("accountId {id}"))
                .unwrap_or_else(|| "(empty)".to_string()),
        ),
        (
            TemplateFieldId::Parent,
            "Parent issue",
            draft
                .parent_key
                .clone()
                .unwrap_or_else(|| "(empty)".to_string()),
        ),
        (
            TemplateFieldId::Sprint,
            &sprint_label,
            sprint_preview(draft, sprint_field),
        ),
        (
            TemplateFieldId::DueDate,
            "Due date",
            draft
                .due_date
                .map(|d| d.format("%Y-%m-%d").to_string())
                .unwrap_or_else(|| "(empty)".to_string()),
        ),
    ];

    rows.into_iter()
        .map(|(id, label, preview)| {
            let has_value = preview != "(empty)";
            let include = match id {
                TemplateFieldId::Summary => true,
                TemplateFieldId::Assignee | TemplateFieldId::Parent => false,
                _ => has_value,
            };
            TemplateFieldRow {
                id,
                label: label.to_string(),
                preview,
                include,
                clear_value: false,
            }
        })
        .collect()
}

fn preview_line(s: &str, max_chars: usize) -> String {
    let one_line: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if one_line.is_empty() {
        return "(empty)".to_string();
    }
    if one_line.chars().count() <= max_chars {
        one_line
    } else {
        let mut end = 0;
        for (i, _) in one_line.char_indices().take(max_chars) {
            end = i;
        }
        format!("{}…", &one_line[..=end])
    }
}

fn sprint_preview(draft: &CreateDraft, sprint_field: Option<&str>) -> String {
    let Some(sf) = sprint_field else {
        return "(empty)".to_string();
    };
    draft
        .extra_fields
        .get(sf)
        .and_then(|v| v.get("name").and_then(|n| n.as_str()))
        .or_else(|| {
            draft
                .extra_fields
                .get(sf)
                .and_then(|v| v.as_array())
                .and_then(|a| a.first())
                .and_then(|s| s.get("name"))
                .and_then(|n| n.as_str())
        })
        .map(String::from)
        .unwrap_or_else(|| "(empty)".to_string())
}

/// Build an `IssueTemplate` from draft data and UI field choices.
pub fn build_issue_template(
    name: &str,
    site_name: &str,
    draft: &CreateDraft,
    rows: &[TemplateFieldRow],
    sprint_field: Option<&str>,
) -> IssueTemplate {
    let include = |id: TemplateFieldId| -> bool {
        rows.iter()
            .find(|r| r.id == id)
            .is_some_and(|r| r.include)
    };
    let clear = |id: TemplateFieldId| -> bool {
        rows.iter()
            .find(|r| r.id == id)
            .is_some_and(|r| r.include && r.clear_value)
    };

    let summary = if include(TemplateFieldId::Summary) && !clear(TemplateFieldId::Summary) {
        summary_for_template(&draft.summary)
    } else if include(TemplateFieldId::Summary) {
        "[fill in summary]".to_string()
    } else {
        "[fill in summary]".to_string()
    };

    let description = if include(TemplateFieldId::Description) && !clear(TemplateFieldId::Description)
    {
        draft.description.clone()
    } else {
        String::new()
    };

    let labels = if include(TemplateFieldId::Labels) && !clear(TemplateFieldId::Labels) {
        draft.labels.clone()
    } else {
        Vec::new()
    };

    let priority = if include(TemplateFieldId::Priority) && !clear(TemplateFieldId::Priority) {
        if draft.priority_name.is_empty() {
            None
        } else {
            Some(draft.priority_name.clone())
        }
    } else {
        None
    };

    let assignee_account_id =
        if include(TemplateFieldId::Assignee) && !clear(TemplateFieldId::Assignee) {
            draft.assignee_account_id.clone()
        } else {
            None
        };

    let parent_key = if include(TemplateFieldId::Parent) && !clear(TemplateFieldId::Parent) {
        draft.parent_key.clone()
    } else {
        None
    };

    let mut extra_fields = HashMap::new();
    if include(TemplateFieldId::Sprint) && !clear(TemplateFieldId::Sprint) {
        if let Some(sf) = sprint_field {
            if let Some(v) = draft.extra_fields.get(sf) {
                if let Ok(tv) = toml::Value::try_from(v.clone()) {
                    extra_fields.insert(sf.to_string(), tv);
                }
            }
        }
    }
    if include(TemplateFieldId::DueDate) && !clear(TemplateFieldId::DueDate) {
        if let Some(d) = draft.due_date {
            extra_fields.insert(
                "duedate".to_string(),
                toml::Value::String(d.format("%Y-%m-%d").to_string()),
            );
        }
    }

    IssueTemplate {
        name: name.trim().to_string(),
        site: Some(site_name.to_string()),
        project: draft.project_key.clone(),
        issue_type: draft.issue_type_name.clone(),
        summary,
        description,
        labels,
        priority,
        assignee_account_id,
        parent_key,
        extra_fields,
    }
}

pub fn template_name_from_key_and_summary(key: &str, summary: &str) -> String {
    let slug: String = summary
        .chars()
        .take(40)
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else if c.is_whitespace() {
                '-'
            } else {
                '-'
            }
        })
        .collect();
    let slug = slug.trim_matches('-');
    if slug.is_empty() {
        key.to_lowercase()
    } else {
        format!("{}-{}", key.to_lowercase(), slug)
    }
}

pub fn summary_for_template(summary: &str) -> String {
    const MAX: usize = 120;
    if summary.chars().count() <= MAX {
        return summary.to_string();
    }
    let mut end = 0;
    for (i, _) in summary.char_indices().take(MAX) {
        end = i;
    }
    format!("{}…", &summary[..=end])
}

/// Append template to config.toml or `create.templates_file`.
pub fn append_issue_template(
    config: &Config,
    template: &IssueTemplate,
    source_key: &str,
) -> Result<PathBuf, String> {
    if config
        .create
        .templates
        .iter()
        .any(|t| t.name == template.name)
    {
        return Err(format!(
            "Template name '{}' already exists — pick another name",
            template.name
        ));
    }

    let external = config.create.templates_file.is_some();
    let path = if let Some(rel) = &config.create.templates_file {
        resolve_templates_path(rel)?
    } else {
        Config::config_path()?
    };

    let block = format_template_block(template, source_key, external);
    write_templates_file(&path, &block, true)?;
    Ok(path)
}

pub fn format_template_block(template: &IssueTemplate, source_key: &str, external_file: bool) -> String {
    let section = if external_file {
        "[[templates]]"
    } else {
        "[[create.templates]]"
    };
    let mut s = format!("# From {source_key} — {}\n", template_picker_label(template));
    s.push_str(section);
    s.push('\n');
    s.push_str(&toml_to_string_pretty(template));
    s.push_str("\n\n");
    s
}

fn toml_to_string_pretty(template: &IssueTemplate) -> String {
    let mut lines = vec![
        format!("name = {}", quote(&template.name)),
        format!(
            "site = {}",
            quote(template.site.as_deref().unwrap_or(""))
        ),
        format!("project = {}", quote(&template.project)),
        format!("issue_type = {}", quote(&template.issue_type)),
        format!("summary = {}", quote(&template.summary)),
    ];
    if !template.description.is_empty() {
        lines.push(format!(
            "description = {}",
            triple_quote(&template.description)
        ));
    }
    if !template.labels.is_empty() {
        let labels: Vec<String> = template.labels.iter().map(|l| quote(l)).collect();
        lines.push(format!("labels = [{}]", labels.join(", ")));
    }
    if let Some(ref p) = template.priority {
        lines.push(format!("priority = {}", quote(p)));
    }
    if let Some(ref a) = template.assignee_account_id {
        lines.push(format!("assignee_account_id = {}", quote(a)));
    }
    if let Some(ref p) = template.parent_key {
        lines.push(format!("parent_key = {}", quote(p)));
    }
    if !template.extra_fields.is_empty() {
        if let Ok(inline) = toml::to_string(&template.extra_fields) {
            lines.push(format!("extra_fields = {inline}"));
        }
    }
    lines.join("\n")
}

fn quote(s: &str) -> String {
    let escaped = s.replace('\\', "\\\\").replace('\"', "\\\"");
    format!("\"{escaped}\"")
}

fn triple_quote(s: &str) -> String {
    format!("'''\n{s}'''")
}

pub fn write_templates_file(path: &Path, content: &str, append: bool) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Cannot create {}: {e}", parent.display()))?;
        }
    }
    if append && path.is_file() {
        let mut existing = std::fs::read_to_string(path)
            .map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
        if !existing.ends_with('\n') {
            existing.push('\n');
        }
        existing.push_str(content);
        std::fs::write(path, existing)
            .map_err(|e| format!("Cannot write {}: {e}", path.display()))?;
    } else {
        std::fs::write(path, content)
            .map_err(|e| format!("Cannot write {}: {e}", path.display()))?;
    }
    Ok(())
}

pub async fn export_issues_to_toml(
    config: &Config,
    site_name: &str,
    keys: &[String],
    jira: &JiraClient,
) -> Result<String, String> {
    let site = config
        .sites
        .iter()
        .find(|s| s.name == site_name)
        .ok_or_else(|| format!("Unknown site '{site_name}' in config"))?;

    let mut out = String::from("# Generated by tick\n");
    out.push_str(&format!("# Site: {} ({})\n\n", site.name, site.base_url));

    for key in keys {
        let template = export_one_issue(jira, site, key).await?;
        out.push_str(&format_template_block(&template, key, true));
    }
    Ok(out)
}

async fn export_one_issue(
    jira: &JiraClient,
    site: &Site,
    key: &str,
) -> Result<IssueTemplate, String> {
    let sprint_field = site.sprint_field.as_deref();
    let issue = jira
        .fetch_issue_for_clone(&site.base_url, key, sprint_field)
        .await?;
    let fields = issue.get("fields").unwrap_or(&issue);

    let project = fields
        .get("project")
        .and_then(|p| p.get("key"))
        .and_then(|k| k.as_str())
        .unwrap_or("")
        .to_string();
    let issue_type = fields
        .get("issuetype")
        .and_then(|p| p.get("name"))
        .and_then(|k| k.as_str())
        .unwrap_or("Task")
        .to_string();
    let summary = fields
        .get("summary")
        .and_then(|s| s.as_str())
        .unwrap_or(key)
        .to_string();

    let mut draft = CreateDraft::default();
    draft.project_key = project.clone();
    draft.issue_type_name = issue_type.clone();
    draft.summary = summary.clone();
    crate::api::create::enrich_draft_from_clone(jira, &mut draft, &issue, sprint_field).await;

    let rows = exportable_field_rows(&draft, sprint_field);
    let name = template_name_from_key_and_summary(key, &summary);
    Ok(build_issue_template(
        &name,
        &site.name,
        &draft,
        &rows,
        sprint_field,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::create::CreateDraft;

    #[test]
    fn summary_for_template_truncates_long() {
        let long = "a".repeat(200);
        let s = summary_for_template(&long);
        assert!(s.ends_with('…'));
        assert!(s.chars().count() <= 125);
    }

    #[test]
    fn template_name_from_key() {
        let n = template_name_from_key_and_summary("HIN-1", "Fix login bug");
        assert!(n.starts_with("hin-1-"));
    }

    #[test]
    fn build_template_clears_assignee() {
        let draft = CreateDraft {
            project_key: "HIN".into(),
            issue_type_name: "Task".into(),
            summary: "Do thing".into(),
            assignee_account_id: Some("abc".into()),
            ..Default::default()
        };
        let mut rows = exportable_field_rows(&draft, None);
        for r in &mut rows {
            if r.id == TemplateFieldId::Assignee {
                r.include = true;
                r.clear_value = true;
            }
        }
        let t = build_issue_template("t1", "site", &draft, &rows, None);
        assert!(t.assignee_account_id.is_none());
        assert_eq!(t.summary, "Do thing");
    }

    #[test]
    fn build_template_omits_description_when_not_included() {
        let draft = CreateDraft {
            project_key: "HIN".into(),
            issue_type_name: "Task".into(),
            summary: "S".into(),
            description: "Long body".into(),
            ..Default::default()
        };
        let mut rows = exportable_field_rows(&draft, None);
        for r in &mut rows {
            if r.id == TemplateFieldId::Description {
                r.include = false;
            }
        }
        let t = build_issue_template("t1", "site", &draft, &rows, None);
        assert!(t.description.is_empty());
    }
}
