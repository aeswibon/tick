use std::sync::Arc;

use clap::{Parser, Subcommand};
use serde::Serialize;

use crate::api::types::Ticket;
use crate::api::JiraClient;
use crate::config::{Config, Site};
use crate::issue_key;

#[derive(Parser)]
pub struct IssueShowArgs {
    pub key: String,
    #[arg(long)]
    pub site: Option<String>,
}

#[derive(Parser)]
pub struct IssueTransitionArgs {
    pub key: String,
    #[arg(long)]
    pub to: String,
    #[arg(long)]
    pub site: Option<String>,
}

#[derive(Subcommand)]
pub enum IssueCommand {
    /// Print issue fields as JSON
    Show(IssueShowArgs),
    /// Apply workflow transition by name
    Transition(IssueTransitionArgs),
}

#[derive(Serialize)]
struct IssueShowJson {
    key: String,
    site: String,
    summary: String,
    status: String,
    priority: String,
    assignee: String,
    labels: Vec<String>,
    url: String,
}

pub async fn run(action: IssueCommand) -> Result<(), Box<dyn std::error::Error>> {
    let result = match action {
        IssueCommand::Show(args) => run_show(args).await,
        IssueCommand::Transition(args) => run_transition(args).await,
    };
    if let Err(e) = result {
        eprintln!("{e}");
        std::process::exit(1);
    }
    Ok(())
}

async fn run_show(args: IssueShowArgs) -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
    let key = parse_key(&args.key)?;
    let site = resolve_site(&config, &key, args.site.as_deref())?;
    let jira = JiraClient::from_config(&config, false)
        .await
        .map_err(|e| format!("Auth error: {e}"))?;
    let ticket = fetch_ticket(&jira, site, &key).await?;
    let dto = IssueShowJson {
        key: ticket.key.clone(),
        site: site.name.clone(),
        summary: ticket.summary,
        status: ticket.status,
        priority: ticket.priority,
        assignee: ticket.assignee,
        labels: ticket.labels,
        url: ticket.link,
    };
    println!("{}", serde_json::to_string_pretty(&dto)?);
    Ok(())
}

async fn run_transition(args: IssueTransitionArgs) -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
    let key = parse_key(&args.key)?;
    let site = resolve_site(&config, &key, args.site.as_deref())?;
    let jira = Arc::new(
        JiraClient::from_config(&config, false)
            .await
            .map_err(|e| format!("Auth error: {e}"))?,
    );
    let base_url = site.base_url.trim_end_matches('/').to_string();
    crate::operations::transition::apply_transition_by_name(&jira, &base_url, &key, &args.to)
        .await
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    println!(
        "{}",
        serde_json::json!({
            "key": key,
            "transition": args.to,
            "site": site.name,
        })
    );
    Ok(())
}

fn parse_key(raw: &str) -> Result<String, String> {
    issue_key::parse_issue_key(raw).ok_or_else(|| format!("Invalid issue key: {raw}"))
}

fn resolve_site<'a>(
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

async fn fetch_ticket(jira: &JiraClient, site: &Site, key: &str) -> Result<Ticket, String> {
    let base_url = site.base_url.trim_end_matches('/');
    let issues = jira
        .bulk_fetch(
            base_url,
            &[key.to_string()],
            site.sprint_field.as_deref(),
            &[],
        )
        .await?;
    let issue = issues
        .into_iter()
        .next()
        .ok_or_else(|| format!("Issue {key} not found"))?;
    Ok(Ticket::from_bulk_fetch(
        issue,
        &site.name,
        base_url,
        site.sprint_field.as_deref(),
        &[],
    ))
}
