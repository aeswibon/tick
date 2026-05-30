//! Config-driven hooks (shell commands on events).

use std::path::PathBuf;
use std::process::Stdio;

use serde::Serialize;
use tokio::process::Command;
use tokio::time::{timeout, Duration};

use crate::api::types::Ticket;
use crate::config::{Config, RefreshHook};

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

fn hook_matches_view(hook: &RefreshHook, view_id: &str) -> bool {
    if hook.views.is_empty() {
        return true;
    }
    hook.views
        .iter()
        .any(|v| v.eq_ignore_ascii_case(view_id))
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

    let json_path = write_hook_json(&payload)?;
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

    let secs = hook.timeout_secs.max(1);
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

fn write_hook_json(payload: &[HookTicketJson]) -> Result<String, String> {
    let dir = std::env::temp_dir().join("tick-hooks");
    std::fs::create_dir_all(&dir).map_err(|e| format!("create hook temp dir: {e}"))?;
    let path: PathBuf = dir.join(format!(
        "refresh-{}-{}.json",
        std::process::id(),
        chrono::Utc::now().timestamp_millis()
    ));
    let body = serde_json::to_string_pretty(payload).map_err(|e| e.to_string())?;
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
