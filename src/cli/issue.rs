use std::sync::Arc;

use clap::{Parser, Subcommand};

use crate::api::types::Ticket;
use crate::api::JiraClient;
use crate::cli::util::{self, IssueJson};
use crate::config::{Config, Site};

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
    let key = util::parse_issue_key_arg(&args.key)?;
    let site = util::resolve_site(&config, &key, args.site.as_deref())?;
    let jira = JiraClient::from_config(&config, false)
        .await
        .map_err(|e| format!("Auth error: {e}"))?;
    let ticket = fetch_ticket(&jira, site, &key).await?;
    let dto = IssueJson::from_ticket(&ticket, &site.name);
    println!("{}", serde_json::to_string_pretty(&dto)?);
    Ok(())
}

async fn run_transition(args: IssueTransitionArgs) -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
    let key = util::parse_issue_key_arg(&args.key)?;
    let site = util::resolve_site(&config, &key, args.site.as_deref())?;
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

async fn fetch_ticket(jira: &JiraClient, site: &Site, key: &str) -> Result<Ticket, String> {
    let base_url = site.base_url.trim_end_matches('/');
    let issues = jira
        .bulk_fetch(
            base_url,
            &[key.to_string()],
            site.sprint_field.as_deref(),
            &[],
            true,
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
        true,
    ))
}
