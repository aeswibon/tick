use serde::Deserialize;
use std::path::{Path, PathBuf};

use super::chord;

/// Plugin API version supported by this tick build (`tick.plugin.toml` `api` field).
pub const API_VERSION: &str = "1";

#[derive(Debug, Clone, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub api: String,
    pub runtime: String,
    pub entry: String,
    #[serde(default)]
    pub capabilities: PluginCapabilities,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct PluginCapabilities {
    #[serde(default)]
    pub filter_tickets: bool,
    /// Chords this plugin handles, e.g. `["ctrl+shift+h"]`.
    #[serde(default)]
    pub on_key: Vec<String>,
    /// Allow `tick.run_transition` / `tick.list_transitions` (delegates to core transition API).
    #[serde(default)]
    pub run_transition: bool,
}

impl PluginManifest {
    pub fn load(path: &Path) -> Result<Self, String> {
        let raw =
            std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
        toml::from_str(&raw).map_err(|e| format!("parse {}: {e}", path.display()))
    }

    pub fn validate(&self, plugin_dir: &Path) -> Result<PathBuf, String> {
        if self.api != API_VERSION {
            return Err(format!(
                "api {} not supported (tick supports {})",
                self.api, API_VERSION
            ));
        }
        if self.runtime != "lua" {
            return Err(format!(
                "runtime '{}' not supported (use lua)",
                self.runtime
            ));
        }
        if self.name.trim().is_empty() {
            return Err("name is required".into());
        }
        if !self.capabilities.filter_tickets
            && self.capabilities.on_key.is_empty()
            && !self.capabilities.run_transition
        {
            return Err("enable filter_tickets, on_key chords, and/or run_transition".into());
        }
        for raw in &self.capabilities.on_key {
            chord::parse_chord(raw).map_err(|e| format!("on_key chord '{raw}': {e}"))?;
        }
        let entry = plugin_dir.join(&self.entry);
        if !entry.is_file() {
            return Err(format!("entry file not found: {}", entry.display()));
        }
        Ok(entry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_filter_and_on_key() {
        let m: PluginManifest = toml::from_str(
            r#"
name = "demo"
version = "0.1.0"
api = "1"
runtime = "lua"
entry = "main.lua"

[capabilities]
filter_tickets = true
on_key = ["ctrl+shift+h"]
"#,
        )
        .unwrap();
        assert!(m.capabilities.filter_tickets);
        assert_eq!(m.capabilities.on_key, vec!["ctrl+shift+h"]);
    }

    #[test]
    fn parses_on_key_only() {
        let m: PluginManifest = toml::from_str(
            r#"
name = "keys"
version = "0.1.0"
api = "1"
runtime = "lua"
entry = "main.lua"

[capabilities]
on_key = ["ctrl+g"]
"#,
        )
        .unwrap();
        assert!(!m.capabilities.filter_tickets);
        assert_eq!(m.capabilities.on_key, vec!["ctrl+g"]);
    }
}
