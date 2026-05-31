use serde::Serialize;

use crate::api::types::Ticket;
use crate::config::{Config, Site};
use crate::issue_key;

#[derive(Serialize)]
pub struct IssueJson {
    pub key: String,
    pub site: String,
    pub summary: String,
    pub status: String,
    pub priority: String,
    pub assignee: String,
    pub labels: Vec<String>,
    pub url: String,
}

impl IssueJson {
    pub fn from_ticket(ticket: &Ticket, site_name: &str) -> Self {
        Self {
            key: ticket.key.clone(),
            site: site_name.to_string(),
            summary: ticket.summary.clone(),
            status: ticket.status.clone(),
            priority: ticket.priority.clone(),
            assignee: ticket.assignee.clone(),
            labels: ticket.labels.clone(),
            url: ticket.link.clone(),
        }
    }
}

pub fn parse_issue_key_arg(raw: &str) -> Result<String, String> {
    issue_key::parse_issue_key(raw).ok_or_else(|| format!("Invalid issue key: {raw}"))
}

pub fn resolve_site<'a>(
    config: &'a Config,
    key: &str,
    site_arg: Option<&str>,
) -> Result<&'a Site, String> {
    if let Some(name) = site_arg {
        return config
            .sites
            .iter()
            .find(|s| s.name == name)
            .ok_or_else(|| format!("Unknown site '{name}'"));
    }
    if config.sites.len() == 1 {
        return Ok(&config.sites[0]);
    }
    Err(format!(
        "Multiple sites configured; pass --site <name> for {key}"
    ))
}

pub fn require_site<'a>(config: &'a Config, name: &str) -> Result<&'a Site, String> {
    config
        .sites
        .iter()
        .find(|s| s.name == name)
        .ok_or_else(|| format!("Unknown site '{name}'"))
}

/// `--body` argument or stdin when omitted (for piped comments).
pub fn read_body_arg(body: Option<String>) -> Result<String, String> {
    if let Some(b) = body {
        if b.trim().is_empty() {
            return Err("comment body must not be empty".into());
        }
        return Ok(b);
    }
    use std::io::Read;
    let mut buf = String::new();
    std::io::stdin()
        .read_to_string(&mut buf)
        .map_err(|e| format!("read stdin: {e}"))?;
    if buf.trim().is_empty() {
        return Err("comment body required: pass --body or pipe text on stdin".into());
    }
    Ok(buf)
}

pub fn parse_keys_list(keys: &[String]) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    for k in keys {
        for part in k.split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            out.push(parse_issue_key_arg(part)?);
        }
    }
    if out.is_empty() {
        return Err("Provide at least one issue key (--keys)".into());
    }
    Ok(out)
}
