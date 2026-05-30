//! Config-driven hooks (shell commands on events).

use std::path::PathBuf;
use std::process::Stdio;

use serde::Serialize;
use tokio::process::Command;
use tokio::time::{timeout, Duration};

use crate::api::types::Ticket;
use crate::batch::{self, BatchOutcome};
use crate::config::{BulkCompleteHook, Config, ConfigReloadHook, MarkHook, RefreshHook};
use crate::config_check::CheckFinding;

#[derive(Clone, Serialize)]
struct HookCheckJson {
    level: String,
    message: String,
}

impl From<&CheckFinding> for HookCheckJson {
    fn from(f: &CheckFinding) -> Self {
        Self {
            level: f.level.to_string(),
            message: f.message.clone(),
        }
    }
}

#[derive(Serialize)]
struct HookTicketJson {
    key: String,
    site: String,
    summary: String,
    status: String,
    assignee: String,
    labels: Vec<String>,
    url: String,
}

/// Fire matching `[[hooks.on_refresh]]` entries (non-blocking spawn per hook).
pub fn fire_on_refresh(config: &Config, view_id: &str, tickets: &[Ticket]) {
    for hook in &config.hooks.on_refresh {
        if !hook_matches_view(hook, view_id) {
            continue;
        }
        let hook = hook.clone();
        let view_id = view_id.to_string();
        let tickets = tickets.to_vec();
        tokio::spawn(async move {
            if let Err(e) = run_on_refresh_hook(&hook, &view_id, &tickets).await {
                eprintln!("[tick hook] {}", e);
            }
        });
    }
}

/// Fire all `[[hooks.on_config_reload]]` entries after a successful `R` config reload.
pub fn fire_on_config_reload(config: &Config, config_path: &str, findings: &[CheckFinding]) {
    if config.hooks.on_config_reload.is_empty() {
        return;
    }
    let error_count = findings
        .iter()
        .filter(|f| f.level == "error")
        .count()
        .to_string();
    let warn_count = findings
        .iter()
        .filter(|f| f.level == "warn")
        .count()
        .to_string();
    let payload: Vec<HookCheckJson> = findings.iter().map(HookCheckJson::from).collect();
    for hook in &config.hooks.on_config_reload {
        let hook = hook.clone();
        let config_path = config_path.to_string();
        let payload = payload.clone();
        let error_count = error_count.clone();
        let warn_count = warn_count.clone();
        tokio::spawn(async move {
            if let Err(e) =
                run_on_config_reload_hook(&hook, &config_path, &payload, &error_count, &warn_count)
                    .await
            {
                eprintln!("[tick hook] {}", e);
            }
        });
    }
}

/// Fire all `[[hooks.on_mark]]` entries when Space adds a bulk mark (not on unmark).
pub fn fire_on_mark(config: &Config, ticket: &Ticket) {
    if config.hooks.on_mark.is_empty() {
        return;
    }
    for hook in &config.hooks.on_mark {
        let hook = hook.clone();
        let ticket = ticket.clone();
        tokio::spawn(async move {
            if let Err(e) = run_on_mark_hook(&hook, &ticket).await {
                eprintln!("[tick hook] {}", e);
            }
        });
    }
}

/// Fire all `[[hooks.on_bulk_complete]]` entries after a bulk action (TUI or CLI).
pub fn fire_on_bulk_complete(config: &Config, label: &str, outcome: &BatchOutcome) {
    if config.hooks.on_bulk_complete.is_empty() {
        return;
    }
    for hook in &config.hooks.on_bulk_complete {
        let hook = hook.clone();
        let label = label.to_string();
        let outcome = batch::bulk_result_payload(&label, outcome);
        tokio::spawn(async move {
            if let Err(e) = run_on_bulk_complete_hook(&hook, &outcome).await {
                eprintln!("[tick hook] {}", e);
            }
        });
    }
}

fn hook_matches_view(hook: &RefreshHook, view_id: &str) -> bool {
    if hook.views.is_empty() {
        return true;
    }
    hook.views.iter().any(|v| v.eq_ignore_ascii_case(view_id))
}

async fn run_on_refresh_hook(
    hook: &RefreshHook,
    view_id: &str,
    tickets: &[Ticket],
) -> Result<(), String> {
    if hook.command.trim().is_empty() {
        return Err("hook command is empty".into());
    }

    let payload: Vec<HookTicketJson> = tickets
        .iter()
        .map(|t| HookTicketJson {
            key: t.key.clone(),
            site: t.site.clone(),
            summary: t.summary.clone(),
            status: t.status.clone(),
            assignee: t.assignee.clone(),
            labels: t.labels.clone(),
            url: t.link.clone(),
        })
        .collect();

    let json_path = write_hook_json_file("refresh", &payload)?;
    let count = tickets.len().to_string();

    let mut cmd = Command::new("sh");
    cmd.arg("-c")
        .arg(&hook.command)
        .env("TICK_VIEW", view_id)
        .env("TICK_JSON_PATH", &json_path)
        .env("TICK_ISSUE_COUNT", &count)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit());

    run_hook_with_timeout(&mut cmd, hook.timeout_secs).await
}

async fn run_on_config_reload_hook(
    hook: &ConfigReloadHook,
    config_path: &str,
    findings: &[HookCheckJson],
    error_count: &str,
    warn_count: &str,
) -> Result<(), String> {
    if hook.command.trim().is_empty() {
        return Err("hook command is empty".into());
    }

    let json_path = write_hook_json_file("config-reload", findings)?;

    let mut cmd = Command::new("sh");
    cmd.arg("-c")
        .arg(&hook.command)
        .env("TICK_CONFIG_PATH", config_path)
        .env("TICK_JSON_PATH", &json_path)
        .env("TICK_CHECK_ERRORS", error_count)
        .env("TICK_CHECK_WARNS", warn_count)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit());

    run_hook_with_timeout(&mut cmd, hook.timeout_secs).await
}

async fn run_on_mark_hook(hook: &MarkHook, ticket: &Ticket) -> Result<(), String> {
    if hook.command.trim().is_empty() {
        return Err("hook command is empty".into());
    }

    let payload = HookTicketJson {
        key: ticket.key.clone(),
        site: ticket.site.clone(),
        summary: ticket.summary.clone(),
        status: ticket.status.clone(),
        assignee: ticket.assignee.clone(),
        labels: ticket.labels.clone(),
        url: ticket.link.clone(),
    };
    let json_path = write_hook_json_file("mark", &payload)?;

    let mut cmd = Command::new("sh");
    cmd.arg("-c")
        .arg(&hook.command)
        .env("TICK_KEY", &ticket.key)
        .env("TICK_SITE", &ticket.site)
        .env("TICK_JSON_PATH", &json_path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit());

    run_hook_with_timeout(&mut cmd, hook.timeout_secs).await
}

async fn run_on_bulk_complete_hook(
    hook: &BulkCompleteHook,
    payload: &batch::BulkResultPayload,
) -> Result<(), String> {
    if hook.command.trim().is_empty() {
        return Err("hook command is empty".into());
    }

    let json_path = write_hook_json_file("bulk", payload)?;
    let ok_count = payload.ok.to_string();
    let fail_count = payload.failed.len().to_string();

    let mut cmd = Command::new("sh");
    cmd.arg("-c")
        .arg(&hook.command)
        .env("TICK_BULK_LABEL", &payload.label)
        .env("TICK_JSON_PATH", &json_path)
        .env("TICK_OK_COUNT", &ok_count)
        .env("TICK_FAIL_COUNT", &fail_count)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit());

    run_hook_with_timeout(&mut cmd, hook.timeout_secs).await
}

async fn run_hook_with_timeout(cmd: &mut Command, timeout_secs: u64) -> Result<(), String> {
    let secs = timeout_secs.max(1);
    let run = cmd.status();
    match timeout(Duration::from_secs(secs), run).await {
        Ok(Ok(status)) if status.success() => Ok(()),
        Ok(Ok(status)) => Err(format!(
            "hook exited with status {}",
            status.code().unwrap_or(-1)
        )),
        Ok(Err(e)) => Err(format!("hook failed to start: {e}")),
        Err(_) => Err(format!("hook timed out after {secs}s")),
    }
}

fn write_hook_json_file(prefix: &str, payload: impl Serialize) -> Result<String, String> {
    let dir = std::env::temp_dir().join("tick-hooks");
    std::fs::create_dir_all(&dir).map_err(|e| format!("create hook temp dir: {e}"))?;
    let path: PathBuf = dir.join(format!(
        "{prefix}-{}-{}.json",
        std::process::id(),
        chrono::Utc::now().timestamp_millis()
    ));
    let body = serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())?;
    std::fs::write(&path, body).map_err(|e| format!("write hook json: {e}"))?;
    Ok(path.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RefreshHook;

    #[test]
    fn hook_matches_empty_views_is_all() {
        let hook = RefreshHook {
            command: "true".into(),
            views: vec![],
            timeout_secs: 30,
        };
        assert!(hook_matches_view(&hook, "assigned"));
        assert!(hook_matches_view(&hook, "My Custom"));
    }

    #[test]
    fn hook_check_json_serializes() {
        let finding = crate::config_check::CheckFinding {
            level: "error",
            message: "bad jql".into(),
        };
        let json = HookCheckJson::from(&finding);
        let body = serde_json::to_string(&json).unwrap();
        assert!(body.contains("error"));
        assert!(body.contains("bad jql"));
    }

    #[test]
    fn hook_matches_filters_views() {
        let hook = RefreshHook {
            command: "true".into(),
            views: vec!["assigned".into(), "mentions".into()],
            timeout_secs: 30,
        };
        assert!(hook_matches_view(&hook, "assigned"));
        assert!(hook_matches_view(&hook, "Assigned"));
        assert!(!hook_matches_view(&hook, "sprint"));
    }
}
