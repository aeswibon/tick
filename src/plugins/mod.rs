//! In-process Lua plugins (`~/.config/tick/plugins/`).

mod chord;
mod manifest;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crossterm::event::{KeyEvent, KeyEventKind, KeyEventState};
use mlua::{Function, HookTriggers, Lua, LuaSerdeExt, Value, VmState};
use serde::{Deserialize, Serialize};

use crate::api::types::Ticket;
use crate::config::Config;

pub use manifest::{PluginManifest, API_VERSION};

const PLUGIN_TIMEOUT: Duration = Duration::from_millis(50);

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

pub enum PluginKeyResult {
    Passthrough,
    Handled,
    HandledWithNotice(String),
}

/// Read-only context for `on_key` handlers.
pub struct PluginContext {
    pub view_name: String,
    pub view_mode: String,
    pub tickets: Vec<PluginTicket>,
}

struct LuaPlugin {
    name: String,
    lua: Lua,
    filter: Option<Function>,
    on_key: Option<Function>,
    /// Canonical chord labels (see `chord::format_key`).
    chords: Vec<String>,
}

/// Loaded plugins and any errors from discovery (shown once at startup / in doctor).
pub struct PluginHost {
    plugins: Vec<LuaPlugin>,
    /// Chord label → plugin indices (load order).
    key_index: HashMap<String, Vec<usize>>,
    pub load_errors: Vec<String>,
}

impl PluginHost {
    pub fn load() -> Self {
        let mut host = Self {
            plugins: Vec::new(),
            key_index: HashMap::new(),
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
            match load_plugin(&plugin_dir, &manifest_path) {
                Ok(p) => {
                    let idx = host.plugins.len();
                    for c in &p.chords {
                        host.key_index.entry(c.clone()).or_default().push(idx);
                    }
                    host.plugins.push(p);
                }
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

    pub fn plugin_count(&self) -> usize {
        self.plugins.len()
    }

    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty() && self.load_errors.is_empty()
    }

    /// Run all filter plugins in directory order.
    pub fn filter_tickets(&self, tickets: &mut Vec<Ticket>) -> Result<(), String> {
        let mut current = std::mem::take(tickets);
        for plugin in self.plugins.iter().filter(|p| p.filter.is_some()) {
            current = plugin.filter_tickets(current)?;
        }
        *tickets = current;
        Ok(())
    }

    /// Dispatch `on_key` to plugins that registered this chord. Returns `true` if handled.
    pub fn try_handle_key(
        &self,
        ctx: &PluginContext,
        key: &KeyEvent,
    ) -> Result<PluginKeyResult, String> {
        let chord_str = chord::format_key(key);
        let Some(indices) = self.key_index.get(&chord_str) else {
            return Ok(PluginKeyResult::Passthrough);
        };
        for &idx in indices {
            let (handled, notice) = self.plugins[idx].call_on_key(&chord_str, ctx)?;
            if handled {
                if let Some(msg) = notice {
                    return Ok(PluginKeyResult::HandledWithNotice(msg));
                }
                return Ok(PluginKeyResult::Handled);
            }
        }
        Ok(PluginKeyResult::Passthrough)
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
        if self.plugins.is_empty() {
            lines.push("  Loaded: none".into());
        } else {
            for p in &self.plugins {
                let mut caps = Vec::new();
                if p.filter.is_some() {
                    caps.push("filter_tickets".into());
                }
                if p.on_key.is_some() {
                    caps.push(format!("on_key [{}]", p.chords.join(", ")));
                }
                lines.push(format!("  Loaded: {} ({})", p.name, caps.join(", ")));
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

impl LuaPlugin {
    fn filter_tickets(&self, tickets: Vec<Ticket>) -> Result<Vec<Ticket>, String> {
        let Some(filter) = &self.filter else {
            return Ok(tickets);
        };
        if tickets.is_empty() {
            return Ok(tickets);
        }

        let payload: Vec<PluginTicket> = tickets.iter().map(PluginTicket::from).collect();
        self.run_with_timeout("filter_tickets", || {
            let input = self
                .lua
                .to_value(&payload)
                .map_err(|e| lua_err(&self.name, e))?;
            let result: Value = filter.call(input).map_err(|e| lua_err(&self.name, e))?;
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
        })
    }

    fn call_on_key(
        &self,
        chord: &str,
        ctx: &PluginContext,
    ) -> Result<(bool, Option<String>), String> {
        let Some(on_key) = &self.on_key else {
            return Ok((false, None));
        };
        self.run_with_timeout("on_key", || {
            register_tick_api(&self.lua, ctx)?;
            let result: String = on_key.call(chord).map_err(|e| lua_err(&self.name, e))?;
            let handled = result.eq_ignore_ascii_case("handled");
            let notice = self
                .lua
                .globals()
                .get::<mlua::Table>("tick")
                .ok()
                .and_then(|t| t.get::<String>("_notice").ok());
            if let Ok(tick) = self.lua.globals().get::<mlua::Table>("tick") {
                let _ = tick.set("_notice", mlua::Value::Nil);
            }
            Ok((handled, notice))
        })
    }

    fn run_with_timeout<T, F>(&self, hook_label: &str, f: F) -> Result<T, String>
    where
        F: FnOnce() -> Result<T, String>,
    {
        let timed_out = Arc::new(AtomicBool::new(false));
        let timed_out_hook = Arc::clone(&timed_out);
        let start = Instant::now();
        let label = hook_label.to_string();

        self.lua.set_hook(
            HookTriggers {
                every_nth_instruction: Some(10_000),
                ..Default::default()
            },
            move |_, _| {
                if start.elapsed() > PLUGIN_TIMEOUT {
                    timed_out_hook.store(true, Ordering::Relaxed);
                    return Err(mlua::Error::RuntimeError(format!(
                        "plugin {label} timed out (50ms)"
                    )));
                }
                Ok(VmState::Continue)
            },
        );

        let out = f();
        self.lua.remove_hook();
        if timed_out.load(Ordering::Relaxed) {
            return Err(format!(
                "plugin \"{}\": {hook_label} timed out (50ms)",
                self.name
            ));
        }
        out
    }
}

fn register_tick_api(lua: &Lua, ctx: &PluginContext) -> Result<(), String> {
    let tick = lua.create_table().map_err(|e| lua_err("tick", e))?;
    tick.set("version", API_VERSION)
        .map_err(|e| lua_err("tick", e))?;

    let view = lua.create_table().map_err(|e| lua_err("tick", e))?;
    view.set("name", ctx.view_name.as_str())
        .map_err(|e| lua_err("tick", e))?;
    view.set("mode", ctx.view_mode.as_str())
        .map_err(|e| lua_err("tick", e))?;
    tick.set("view", view).map_err(|e| lua_err("tick", e))?;

    let tickets: Value = lua.to_value(&ctx.tickets).map_err(|e| lua_err("tick", e))?;
    tick.set("tickets", tickets)
        .map_err(|e| lua_err("tick", e))?;

    lua.globals()
        .set("tick", tick)
        .map_err(|e| lua_err("tick", e))?;
    Ok(())
}

fn load_plugin(plugin_dir: &Path, manifest_path: &Path) -> Result<LuaPlugin, String> {
    let manifest = PluginManifest::load(manifest_path)?;
    let entry = manifest.validate(plugin_dir)?;
    let script =
        std::fs::read_to_string(&entry).map_err(|e| format!("read {}: {e}", entry.display()))?;

    let lua = Lua::new();
    lua.load(&script)
        .set_name(format!("@{}", entry.display()))
        .exec()
        .map_err(|e| format!("{}: {e}", manifest.name))?;

    let filter = if manifest.capabilities.filter_tickets {
        Some(
            lua.globals()
                .get("filter_tickets")
                .map_err(|_| format!("{}: global filter_tickets() not defined", manifest.name))?,
        )
    } else {
        None
    };

    let on_key = if !manifest.capabilities.on_key.is_empty() {
        Some(
            lua.globals()
                .get("on_key")
                .map_err(|_| format!("{}: global on_key() not defined", manifest.name))?,
        )
    } else {
        None
    };

    let mut chords = Vec::new();
    for raw in &manifest.capabilities.on_key {
        let parsed = chord::parse_chord(raw)?;
        let label = chord::format_key(&KeyEvent {
            code: parsed.code,
            modifiers: parsed.modifiers,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        });
        chords.push(label);
    }

    Ok(LuaPlugin {
        name: manifest.name,
        lua,
        filter,
        on_key,
        chords,
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
        let plugin = LuaPlugin {
            name: "test".into(),
            lua,
            filter: Some(filter),
            on_key: None,
            chords: Vec::new(),
        };

        let tickets = vec![ticket("A-1", "Epic"), ticket("A-2", "Story")];
        let out = plugin.filter_tickets(tickets).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].key, "A-2");
    }

    #[test]
    fn on_key_handled() {
        let script = r#"
function on_key(chord)
  if chord == "ctrl+shift+h" then
    return "handled"
  end
  return "passthrough"
end
"#;
        let lua = Lua::new();
        lua.load(script).exec().unwrap();
        let on_key: Function = lua.globals().get("on_key").unwrap();
        let plugin = LuaPlugin {
            name: "test".into(),
            lua,
            filter: None,
            on_key: Some(on_key),
            chords: vec!["ctrl+shift+h".into()],
        };
        let ctx = PluginContext {
            view_name: "assigned".into(),
            view_mode: "my_issues".into(),
            tickets: vec![],
        };
        assert_eq!(
            plugin.call_on_key("ctrl+shift+h", &ctx).unwrap(),
            (true, None)
        );
        assert_eq!(plugin.call_on_key("ctrl+g", &ctx).unwrap(), (false, None));
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
