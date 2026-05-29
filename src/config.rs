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

fn default_theme() -> String {
    "default".to_string()
}

fn default_max_results() -> u32 {
    50
}

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
        let config: Config = toml::from_str(&contents)
            .map_err(|e| format!("Invalid config: {}", e))?;
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.email.trim().is_empty() {
            return Err("config: email must not be empty".into());
        }
        if self.token.trim().is_empty() {
            return Err("config: token must not be empty".into());
        }
        if self.sites.is_empty() {
            return Err("config: at least one [[sites]] entry is required".into());
        }
        for site in &self.sites {
            if site.name.trim().is_empty() {
                return Err("config: site name must not be empty".into());
            }
            let url = site.base_url.trim();
            if url.is_empty() {
                return Err(format!("config: base_url for site '{}' must not be empty", site.name));
            }
            if !url.starts_with("https://") {
                return Err(format!(
                    "config: base_url for site '{}' must start with https://",
                    site.name
                ));
            }
        }
        Ok(())
    }

    pub fn create_default_config() -> Result<(), String> {
        let path = Self::config_path()?;
        if path.exists() {
            return Ok(());
        }
        let parent = path
            .parent()
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
name = "my-team"
base_url = "https://my-team.atlassian.net"
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
        assert!(cfg.validate().is_ok());
        assert_eq!(cfg.max_results, 25);
    }

    #[test]
    fn rejects_empty_sites() {
        let raw = r#"
email = "a@b.com"
token = "secret"
sites = []
"#;
        let cfg: Config = toml::from_str(raw).unwrap();
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn rejects_non_https_base_url() {
        let raw = r#"
email = "a@b.com"
token = "secret"

[[sites]]
name = "one"
base_url = "http://one.atlassian.net"
"#;
        let cfg: Config = toml::from_str(raw).unwrap();
        assert!(cfg.validate().is_err());
    }
}
