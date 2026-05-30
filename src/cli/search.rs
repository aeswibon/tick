use clap::Parser;
use serde::Serialize;

use crate::api::{self, JiraClient};
use crate::cli::util::IssueJson;
use crate::columns::Column;
use crate::config::Config;

#[derive(Parser)]
pub struct SearchArgs {
    /// JQL query (include ORDER BY if order matters)
    #[arg(long)]
    pub jql: String,
    /// Limit to one [[sites]].name
    #[arg(long)]
    pub site: Option<String>,
    /// Suppress progress messages on stderr
    #[arg(long)]
    pub quiet: bool,
}

#[derive(Serialize)]
struct SearchResult {
    issues: Vec<IssueJson>,
    warnings: Vec<String>,
}

pub async fn run(args: SearchArgs) -> Result<(), String> {
    let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
    let jira = JiraClient::from_config(&config, false)
        .await
        .map_err(|e| format!("Auth error: {e}"))?;
    let custom_ids = Column::custom_field_ids(&Column::resolve(config.columns.as_deref()));
    if !args.quiet {
        eprintln!("Searching Jira…");
    }
    let (tickets, warnings) =
        api::fetch_all(&jira, &config, &args.jql, args.site.as_deref(), &custom_ids).await;
    let issues: Vec<IssueJson> = tickets
        .iter()
        .map(|t| IssueJson::from_ticket(t, &t.site))
        .collect();
    if !warnings.is_empty() && !args.quiet {
        for w in &warnings {
            eprintln!("warning: {w}");
        }
    }
    let out = SearchResult { issues, warnings };
    println!(
        "{}",
        serde_json::to_string_pretty(&out).map_err(|e| e.to_string())?
    );
    Ok(())
}
