//! Rewrite `[[create.templates]]` / `templates_file` after template manager edits.

use crate::config::Config;
use crate::template_export::{format_template_block, resolve_templates_path, write_templates_file};

/// Persist all in-memory templates to disk (templates file or config.toml).
pub fn save_all_templates(config: &Config) -> Result<std::path::PathBuf, String> {
    if let Some(rel) = &config.create.templates_file {
        let path = resolve_templates_path(rel)?;
        let mut body = String::from("# Managed by tick template editor\n\n");
        for template in &config.create.templates {
            body.push_str(&format_template_block(template, &template.name, true));
        }
        write_templates_file(&path, &body, false)?;
        return Ok(path);
    }

    let path = Config::config_path()?;
    let raw = std::fs::read_to_string(&path)
        .map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
    let stripped = strip_inline_template_blocks(&raw);
    let mut body = stripped;
    if !body.ends_with('\n') {
        body.push('\n');
    }
    for template in &config.create.templates {
        body.push_str(&format_template_block(template, &template.name, false));
    }
    std::fs::write(&path, body).map_err(|e| format!("Cannot write {}: {e}", path.display()))?;
    Ok(path)
}

/// Remove `[[create.templates]]` blocks from config text.
fn strip_inline_template_blocks(raw: &str) -> String {
    let mut out = String::new();
    let mut skip = false;
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed == "[[create.templates]]" {
            skip = true;
            continue;
        }
        if skip {
            let ends_template_block = trimmed.starts_with("[[")
                || (trimmed.starts_with('[')
                    && trimmed.ends_with(']')
                    && !trimmed.starts_with("[["));
            if ends_template_block {
                skip = false;
            } else {
                continue;
            }
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

pub fn remove_template(config: &mut Config, name: &str) -> Result<(), String> {
    let before = config.create.templates.len();
    config.create.templates.retain(|t| t.name != name);
    if config.create.templates.len() == before {
        return Err(format!("Unknown template '{name}'"));
    }
    save_all_templates(config)?;
    Ok(())
}

pub fn update_template_field(
    config: &mut Config,
    name: &str,
    field: TemplateEditField,
    value: String,
) -> Result<(), String> {
    let template = config
        .create
        .templates
        .iter_mut()
        .find(|t| t.name == name)
        .ok_or_else(|| format!("Unknown template '{name}'"))?;
    match field {
        TemplateEditField::Summary => template.summary = value,
        TemplateEditField::Project => template.project = value,
        TemplateEditField::IssueType => template.issue_type = value,
        TemplateEditField::Description => template.description = value,
    }
    template.validate_fields()?;
    save_all_templates(config)?;
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateEditField {
    Summary,
    Project,
    IssueType,
    Description,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_removes_template_blocks() {
        let raw = r#"
email = "a@b.com"

[[create.templates]]
name = "old"
project = "X"
issue_type = "Task"
summary = "s"

[views]
assigned = "x"
"#;
        let out = strip_inline_template_blocks(raw);
        assert!(!out.contains("[[create.templates]]"));
        assert!(!out.contains("name = \"old\""));
        assert!(out.contains("[views]"));
    }
}
