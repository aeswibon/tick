use serde::Deserialize;
use std::path::{Path, PathBuf};

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
        if !self.capabilities.filter_tickets {
            return Err("capabilities.filter_tickets must be true for tick v0.21 plugins".into());
        }
        if self.name.trim().is_empty() {
            return Err("name is required".into());
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
    fn parses_minimal_manifest() {
        let m: PluginManifest = toml::from_str(
            r#"
name = "demo"
version = "0.1.0"
api = "1"
runtime = "lua"
entry = "main.lua"

[capabilities]
filter_tickets = true
"#,
        )
        .unwrap();
        assert_eq!(m.name, "demo");
        assert!(m.capabilities.filter_tickets);
    }
}
