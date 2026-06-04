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

#[derive(Parser)]
pub struct IssueCommentArgs {
    pub key: String,
    /// Comment text (markdown). Omit to read from stdin.
    #[arg(long)]
    pub body: Option<String>,
    /// Attach a file (repeatable). Images upload inline; other files as issue attachments.
    #[arg(long = "attach", value_name = "PATH")]
    attach: Vec<std::path::PathBuf>,
    #[arg(long)]
    pub site: Option<String>,
}

#[derive(Parser)]
pub struct IssueAssignArgs {
    pub key: String,
    #[arg(long)]
    pub site: Option<String>,
    /// Assign to the current Jira user
    #[arg(long, conflicts_with = "unassign")]
    pub me: bool,
    /// Clear assignee
    #[arg(long, conflicts_with = "me")]
    pub unassign: bool,
}

#[derive(Parser)]
pub struct IssueWatchArgs {
    pub key: String,
    #[arg(long)]
    pub site: Option<String>,
}

#[derive(Subcommand)]
pub enum IssueCommand {
    /// Print issue fields as JSON
    Show(IssueShowArgs),
    /// Apply workflow transition by name
    Transition(IssueTransitionArgs),
    /// Add a comment (markdown; @mentions not resolved from CLI)
    Comment(IssueCommentArgs),
    /// Assign to current user or clear assignee
    Assign(IssueAssignArgs),
    /// Add current user as watcher
    Watch(IssueWatchArgs),
    /// Remove current user from watchers
    Unwatch(IssueWatchArgs),
}

pub async fn run(action: IssueCommand) -> Result<(), Box<dyn std::error::Error>> {
    let result = match action {
        IssueCommand::Show(args) => run_show(args).await,
        IssueCommand::Transition(args) => run_transition(args).await,
        IssueCommand::Comment(args) => run_comment(args).await,
        IssueCommand::Assign(args) => run_assign(args).await,
        IssueCommand::Watch(args) => run_watch(args, false).await,
        IssueCommand::Unwatch(args) => run_watch(args, true).await,
    };
    if let Err(e) = result {
        eprintln!("{e}");
        std::process::exit(1);
    }
    Ok(())
}

struct IssueCtx {
    jira: Arc<JiraClient>,
    site_name: String,
    base_url: String,
    key: String,
}

async fn load_issue_ctx(key_raw: &str, site_arg: Option<&str>) -> Result<IssueCtx, String> {
    let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
    let key = util::parse_issue_key_arg(key_raw)?;
    let site = util::resolve_site(&config, &key, site_arg)?;
    let jira = Arc::new(
        JiraClient::from_config(&config, false)
            .await
            .map_err(|e| format!("Auth error: {e}"))?,
    );
    Ok(IssueCtx {
        jira,
        site_name: site.name.clone(),
        base_url: site.base_url.trim_end_matches('/').to_string(),
        key,
    })
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
    let ctx = load_issue_ctx(&args.key, args.site.as_deref()).await?;
    crate::operations::transition::apply_transition_by_name(
        &ctx.jira,
        &ctx.base_url,
        &ctx.key,
        &args.to,
    )
    .await
    .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    print_issue_ok(&ctx, "transition", serde_json::json!({ "to": args.to }));
    Ok(())
}

async fn run_comment(args: IssueCommentArgs) -> Result<(), Box<dyn std::error::Error>> {
    let body = util::read_body_arg(args.body)?;
    let ctx = load_issue_ctx(&args.key, args.site.as_deref()).await?;
    for path in &args.attach {
        if !path.is_file() {
            return Err(format!("Not a file: {}", path.display()).into());
        }
    }
    ctx.jira
        .add_comment_with_attachments(&ctx.base_url, &ctx.key, &body, &[], &args.attach)
        .await?;
    print_issue_ok(
        &ctx,
        "comment",
        serde_json::json!({
            "chars": body.len(),
            "attachments": args.attach.len(),
        }),
    );
    Ok(())
}

async fn run_assign(args: IssueAssignArgs) -> Result<(), Box<dyn std::error::Error>> {
    if !args.me && !args.unassign {
        return Err("Specify --me or --unassign".into());
    }
    let ctx = load_issue_ctx(&args.key, args.site.as_deref()).await?;
    if args.unassign {
        ctx.jira.unassign(&ctx.base_url, &ctx.key).await?;
        print_issue_ok(&ctx, "unassign", serde_json::json!({}));
    } else {
        let account_id = ctx.jira.current_user_account_id(&ctx.base_url).await?;
        ctx.jira
            .assign_to_account(&ctx.base_url, &ctx.key, &account_id)
            .await?;
        print_issue_ok(
            &ctx,
            "assign",
            serde_json::json!({ "assignee": "me", "accountId": account_id }),
        );
    }
    Ok(())
}

async fn run_watch(args: IssueWatchArgs, unwatch: bool) -> Result<(), Box<dyn std::error::Error>> {
    let ctx = load_issue_ctx(&args.key, args.site.as_deref()).await?;
    if unwatch {
        ctx.jira.unwatch_issue(&ctx.base_url, &ctx.key).await?;
        print_issue_ok(&ctx, "unwatch", serde_json::json!({}));
    } else {
        ctx.jira.watch_issue(&ctx.base_url, &ctx.key).await?;
        print_issue_ok(&ctx, "watch", serde_json::json!({}));
    }
    Ok(())
}

fn print_issue_ok(ctx: &IssueCtx, action: &str, extra: serde_json::Value) {
    println!(
        "{}",
        serde_json::json!({
            "key": ctx.key,
            "site": ctx.site_name,
            "action": action,
            "ok": true,
            "detail": extra,
        })
    );
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_body_from_option() {
        assert_eq!(util::read_body_arg(Some("hello".into())).unwrap(), "hello");
    }

    #[test]
    fn read_body_rejects_empty_option() {
        assert!(util::read_body_arg(Some("  ".into())).is_err());
    }
}
