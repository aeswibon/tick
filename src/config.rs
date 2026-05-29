use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Site {
    pub name: String,
    pub base_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub email: String,
    pub token: String,
    pub sites: Vec<Site>,
    #[serde(default)]
    pub columns: Option<Vec<String>>,
    #[serde(default = "default_max_results")]
    pub max_results: u32,
    #[serde(default = "default_theme")]
    pub theme: String,
}

fn default_theme() -> String { "default".to_string() }

fn default_max_results() -> u32 { 50 }

impl Config {
    pub fn config_path() -> Result<PathBuf, String> {
        let base = dirs::config_dir()
            .ok_or_else(|| "Cannot determine config directory".to_string())?;
        Ok(base.join("tick").join("config.toml"))
    }

    pub fn load() -> Result<Self, String> {
        let path = Self::config_path()?;
        let contents = fs::read_to_string(&path)
            .map_err(|e| format!("Cannot read {}: {}", path.display(), e))?;
        toml::from_str(&contents)
            .map_err(|e| format!("Invalid config: {}", e))
    }

    pub fn create_default_config() -> Result<(), String> {
        let path = Self::config_path()?;
        if path.exists() {
            return Ok(());
        }
        let parent = path.parent()
            .ok_or_else(|| "Config path has no parent directory".to_string())?;
        fs::create_dir_all(parent)
            .map_err(|e| format!("Cannot create config dir {}: {}", parent.display(), e))?;
        let default = r#"email = "you@example.com"
token = "your-api-token"
max_results = 50
theme = "default"

# Optional table columns (ids: site, key, type, status, priority, age, due, assignee, reporter, summary)
# columns = ["site", "key", "type", "status", "priority", "age", "due", "assignee", "reporter"]

[[sites]]
name = "zeta-tm"
base_url = "https://zeta-tm.atlassian.net"

[[sites]]
name = "zeta-saas"
base_url = "https://zeta-saas.atlassian.net"
"#;
        fs::write(&path, default)
            .map_err(|e| format!("Cannot write {}: {}", path.display(), e))?;
        println!("Default config created at {}", path.display());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_config() {
        let raw = r#"
email = "a@b.com"
token = "secret"
max_results = 25
theme = "dracula"

[[sites]]
name = "one"
base_url = "https://one.atlassian.net"
"#;
        let cfg: Config = toml::from_str(raw).unwrap();
        assert_eq!(cfg.email, "a@b.com");
        assert_eq!(cfg.max_results, 25);
        assert_eq!(cfg.theme, "dracula");
        assert_eq!(cfg.sites.len(), 1);
    }

    #[test]
    fn parses_optional_columns() {
        let raw = r#"
email = "a@b.com"
token = "secret"
columns = ["key", "summary"]

[[sites]]
name = "one"
base_url = "https://one.atlassian.net"
"#;
        let cfg: Config = toml::from_str(raw).unwrap();
        assert_eq!(cfg.columns.as_ref().map(|c| c.len()), Some(2));
    }
}
