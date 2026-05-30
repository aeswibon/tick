//! In-process Lua plugins (`~/.config/tick/plugins/`). Track C.1: `filter_tickets` only.

mod manifest;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use mlua::{Function, HookTriggers, Lua, LuaSerdeExt, Value, VmState};
use serde::{Deserialize, Serialize};

use crate::api::types::Ticket;
use crate::config::Config;

pub use manifest::{PluginManifest, API_VERSION};

const FILTER_TIMEOUT: Duration = Duration::from_millis(50);

/// Ticket shape exposed to Lua (`filter_tickets` in / out).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginTicket {
    pub key: String,
    pub site: String,
    pub summary: String,
    pub status: String,
    pub priority: String,
    pub assignee: String,
    pub issue_type: String,
    pub labels: Vec<String>,
    pub url: String,
}

impl From<&Ticket> for PluginTicket {
    fn from(t: &Ticket) -> Self {
        Self {
            key: t.key.clone(),
            site: t.site.clone(),
            summary: t.summary.clone(),
            status: t.status.clone(),
            priority: t.priority.clone(),
            assignee: t.assignee.clone(),
            issue_type: t.issue_type.clone(),
            labels: t.labels.clone(),
            url: t.link.clone(),
        }
    }
}

struct LuaFilterPlugin {
    name: String,
    lua: Lua,
    filter: Function,
}

/// Loaded plugins and any errors from discovery (shown once at startup / in doctor).
pub struct PluginHost {
    filters: Vec<LuaFilterPlugin>,
    pub load_errors: Vec<String>,
}

impl PluginHost {
    pub fn load() -> Self {
        let mut host = Self {
            filters: Vec::new(),
            load_errors: Vec::new(),
        };
        let dir = match plugins_dir() {
            Ok(d) => d,
            Err(e) => {
                host.load_errors.push(e);
                return host;
            }
        };
        if !dir.is_dir() {
            return host;
        }

        let mut entries: Vec<PathBuf> = match std::fs::read_dir(&dir) {
            Ok(rd) => rd
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.is_dir())
                .collect(),
            Err(e) => {
                host.load_errors
                    .push(format!("plugins dir {}: {e}", dir.display()));
                return host;
            }
        };
        entries.sort();

        for plugin_dir in entries {
            let manifest_path = plugin_dir.join("tick.plugin.toml");
            if !manifest_path.is_file() {
                continue;
            }
            match load_filter_plugin(&plugin_dir, &manifest_path) {
                Ok(p) => host.filters.push(p),
                Err(e) => {
                    let label = plugin_dir
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("?");
                    host.load_errors.push(format!("{label}: {e}"));
                }
            }
        }
        host
    }

    pub fn filter_count(&self) -> usize {
        self.filters.len()
    }

    pub fn is_empty(&self) -> bool {
        self.filters.is_empty() && self.load_errors.is_empty()
    }

    /// Run all filter plugins in directory order. Preserves `Ticket` rows returned by plugins.
    pub fn filter_tickets(&self, tickets: &mut Vec<Ticket>) -> Result<(), String> {
        let mut current = std::mem::take(tickets);
        for plugin in &self.filters {
            current = plugin.filter(current)?;
        }
        *tickets = current;
        Ok(())
    }

    pub fn doctor_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        let dir = match plugins_dir() {
            Ok(d) => d,
            Err(e) => {
                lines.push(format!("  Plugins: {e}"));
                return lines;
            }
        };
        lines.push(format!("  Plugins dir: {}", dir.display()));
        if self.filters.is_empty() {
            lines.push("  Loaded: none".into());
        } else {
            for p in &self.filters {
                lines.push(format!("  Loaded: {} (filter_tickets)", p.name));
            }
        }
        for e in &self.load_errors {
            lines.push(format!("  Error: {e}"));
        }
        if dir.is_dir() {
            if let Ok(rd) = std::fs::read_dir(&dir) {
                let dirs: Vec<_> = rd
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().is_dir())
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .collect();
                if dirs.is_empty() {
                    lines.push("  (no plugin subdirectories)".into());
                }
            }
        } else {
            lines.push("  (directory does not exist yet)".into());
        }
        lines
    }
}

impl LuaFilterPlugin {
    fn filter(&self, tickets: Vec<Ticket>) -> Result<Vec<Ticket>, String> {
        if tickets.is_empty() {
            return Ok(tickets);
        }

        let payload: Vec<PluginTicket> = tickets.iter().map(PluginTicket::from).collect();
        let timed_out = Arc::new(AtomicBool::new(false));
        let timed_out_hook = Arc::clone(&timed_out);
        let start = Instant::now();

        self.lua.set_hook(
            HookTriggers {
                every_nth_instruction: Some(10_000),
                ..Default::default()
            },
            move |_, _| {
                if start.elapsed() > FILTER_TIMEOUT {
                    timed_out_hook.store(true, Ordering::Relaxed);
                    return Err(mlua::Error::RuntimeError(
                        "plugin filter_tickets timed out (50ms)".into(),
                    ));
                }
                Ok(VmState::Continue)
            },
        );

        let input: Value = self
            .lua
            .to_value(&payload)
            .map_err(|e| lua_err(&self.name, e))?;

        let result: Value = self
            .filter
            .call(input)
            .map_err(|e| lua_err(&self.name, e))?;

        self.lua.remove_hook();
        if timed_out.load(Ordering::Relaxed) {
            return Err(format!(
                "plugin \"{}\": filter_tickets timed out (50ms)",
                self.name
            ));
        }

        let returned: Vec<PluginTicket> = self
            .lua
            .from_value(result)
            .map_err(|e| lua_err(&self.name, e))?;

        let mut out = Vec::with_capacity(returned.len());
        for r in &returned {
            if let Some(t) = tickets.iter().find(|t| t.key == r.key && t.site == r.site) {
                out.push(t.clone());
            }
        }
        Ok(out)
    }
}

fn load_filter_plugin(plugin_dir: &Path, manifest_path: &Path) -> Result<LuaFilterPlugin, String> {
    let manifest = PluginManifest::load(manifest_path)?;
    let entry = manifest.validate(plugin_dir)?;
    let script =
        std::fs::read_to_string(&entry).map_err(|e| format!("read {}: {e}", entry.display()))?;

    let lua = Lua::new();
    lua.load(&script)
        .set_name(format!("@{}", entry.display()))
        .exec()
        .map_err(|e| format!("{}: {e}", manifest.name))?;

    let filter: Function = lua
        .globals()
        .get("filter_tickets")
        .map_err(|_| format!("{}: global filter_tickets() not defined", manifest.name))?;

    Ok(LuaFilterPlugin {
        name: manifest.name,
        lua,
        filter,
    })
}

fn lua_err(name: &str, e: mlua::Error) -> String {
    format!("plugin \"{name}\": {e}")
}

pub fn plugins_dir() -> Result<PathBuf, String> {
    Ok(Config::config_dir()?.join("plugins"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_tickets_drops_epics() {
        let script = r#"
function filter_tickets(tickets)
  local out = {}
  for _, t in ipairs(tickets) do
    if t.issue_type ~= "Epic" then
      table.insert(out, t)
    end
  end
  return out
end
"#;
        let lua = Lua::new();
        lua.load(script).exec().unwrap();
        let filter: Function = lua.globals().get("filter_tickets").unwrap();
        let plugin = LuaFilterPlugin {
            name: "test".into(),
            lua,
            filter,
        };

        let tickets = vec![ticket("A-1", "Epic"), ticket("A-2", "Story")];
        let out = plugin.filter(tickets).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].key, "A-2");
    }

    fn ticket(key: &str, issue_type: &str) -> Ticket {
        Ticket {
            key: key.into(),
            site: "test".into(),
            issue_type: issue_type.into(),
            status: "To Do".into(),
            status_color: String::new(),
            priority: String::new(),
            ageing_days: 0,
            due_date: None,
            assignee: String::new(),
            reporter: String::new(),
            summary: "S".into(),
            link: format!("https://example.com/browse/{key}"),
            description: None,
            description_adf: None,
            latest_comment: None,
            all_comments: Vec::new(),
            parent_key: None,
            parent_summary: None,
            labels: Vec::new(),
            sprint_name: None,
            project_key: "A".into(),
            custom_fields: Default::default(),
            detail_loaded: false,
        }
    }
}
