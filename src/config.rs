use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::api::agile::BoardConfig;
use crate::view_mode::ViewMode;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Site {
    pub name: String,
    pub base_url: String,
    /// Jira field id for sprint (e.g. customfield_10020). Use `tick --doctor` to discover.
    #[serde(default)]
    pub sprint_field: Option<String>,
    /// Default agile board id for sprint moves (`M`). See `tick --doctor`.
    #[serde(default)]
    pub board_id: Option<u64>,
    /// Per-project board overrides: `boards = { DEMO = 7, WEB = 12 }`
    #[serde(default)]
    pub boards: HashMap<String, u64>,
}

impl Site {
    pub fn board_config(&self) -> BoardConfig {
        BoardConfig {
            default_board_id: self.board_id,
            project_boards: self.boards.clone(),
        }
    }

    pub fn is_board_configured(&self, board_id: u64, project_key: Option<&str>) -> bool {
        if self.board_id == Some(board_id) {
            return true;
        }
        if let Some(pk) = project_key {
            return self.boards.get(pk) == Some(&board_id);
        }
        false
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ViewQueries {
    pub assigned: Option<String>,
    pub updated: Option<String>,
    pub mentions: Option<String>,
    pub watched: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub email: String,
    #[serde(default)]
    pub token: String,
    pub sites: Vec<Site>,
    #[serde(default)]
    pub columns: Option<Vec<String>>,
    #[serde(default = "default_max_results")]
    pub max_results: u32,
    #[serde(default = "default_page_size")]
    pub page_size: u32,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default)]
    pub views: ViewQueries,
    /// Desktop notification when a background or scheduled refresh finds new issues.
    #[serde(default)]
    pub notify_on_refresh: bool,
    #[serde(skip)]
    pub view_jql: HashMap<ViewMode, String>,
}

fn default_theme() -> String {
    "default".to_string()
}

fn default_max_results() -> u32 {
    50
}

fn default_page_size() -> u32 {
    10
}

impl Config {
    pub fn config_dir() -> Result<PathBuf, String> {
        let base =
            dirs::config_dir().ok_or_else(|| "Cannot determine config directory".to_string())?;
        Ok(base.join("tick"))
    }

    pub fn config_path() -> Result<PathBuf, String> {
        Ok(Self::config_dir()?.join("config.toml"))
    }

    pub fn token_path() -> Result<PathBuf, String> {
        Ok(Self::config_dir()?.join("token"))
    }

    pub fn load() -> Result<Self, String> {
        let path = Self::config_path()?;
        let contents = fs::read_to_string(&path)
            .map_err(|e| format!("Cannot read {}: {}", path.display(), e))?;
        let mut config: Config =
            toml::from_str(&contents).map_err(|e| format!("Invalid config: {}", e))?;
        config.token = Self::resolve_token(&config.token)?;
        config.view_jql = Self::build_view_jql(&config.views);
        config.validate()?;
        Ok(config)
    }

    /// TICK_TOKEN env → ~/.config/tick/token file → config.toml `token`
    pub fn resolve_token(config_token: &str) -> Result<String, String> {
        if let Ok(t) = std::env::var("TICK_TOKEN") {
            let t = t.trim();
            if !t.is_empty() {
                return Ok(t.to_string());
            }
        }
        let path = Self::token_path()?;
        if path.is_file() {
            let t = fs::read_to_string(&path)
                .map_err(|e| format!("Cannot read {}: {}", path.display(), e))?;
            let t = t.trim();
            if !t.is_empty() {
                return Ok(t.to_string());
            }
        }
        let t = config_token.trim();
        if t.is_empty() || t == "your-api-token" {
            return Err(
                "No API token: set TICK_TOKEN, create ~/.config/tick/token, or set token in config.toml"
                    .into(),
            );
        }
        Ok(t.to_string())
    }

    pub(crate) fn build_view_jql(views: &ViewQueries) -> HashMap<ViewMode, String> {
        ViewMode::all()
            .into_iter()
            .map(|mode| {
                let jql = match mode {
                    ViewMode::MyIssues => views.assigned.as_deref(),
                    ViewMode::Updated => views.updated.as_deref(),
                    ViewMode::Mentions => views.mentions.as_deref(),
                    ViewMode::Watching => views.watched.as_deref(),
                };
                let s = jql
                    .map(String::from)
                    .unwrap_or_else(|| mode.default_jql().to_string());
                (mode, s)
            })
            .collect()
    }

    pub fn jql_for(&self, mode: ViewMode) -> &str {
        self.view_jql
            .get(&mode)
            .map(String::as_str)
            .unwrap_or(mode.default_jql())
    }

    /// Apply CLI overrides and re-validate (call after `load()`).
    pub fn apply_cli_overrides(
        &mut self,
        max_results: Option<u32>,
        page_size: Option<u32>,
    ) -> Result<(), String> {
        if let Some(mr) = max_results {
            self.max_results = mr;
        }
        if let Some(ps) = page_size {
            self.page_size = ps;
        }
        if max_results.is_some() || page_size.is_some() {
            self.validate()?;
        }
        Ok(())
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.email.trim().is_empty() {
            return Err("config: email must not be empty".into());
        }
        if self.token.trim().is_empty() {
            return Err("config: token must not be empty".into());
        }
        if self.page_size == 0 {
            return Err("config: page_size must be at least 1".into());
        }
        if self.page_size > 500 {
            return Err("config: page_size must be at most 500".into());
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
                return Err(format!(
                    "config: base_url for site '{}' must not be empty",
                    site.name
                ));
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
# token = "your-api-token"   # or use ~/.config/tick/token or TICK_TOKEN env
max_results = 50
page_size = 10   # rows to scroll with [ and ]
theme = "default"
# notify_on_refresh = true   # desktop alert when new issues appear on refresh

# Optional custom JQL per view (defaults shown commented)
# [views]
# assigned = "assignee = currentUser() AND statusCategory != Done ORDER BY updated DESC"
# updated = "assignee = currentUser() AND statusCategory != Done AND updated >= -7d ORDER BY updated DESC"
# mentions = "comment ~ currentUser() AND statusCategory != Done ORDER BY updated DESC"
# watched = "watcher = currentUser() AND statusCategory != Done ORDER BY updated DESC"
# sprint = "sprint in openSprints() AND assignee = currentUser() ORDER BY updated DESC"

# Optional table columns (ids: site, key, type, status, priority, age, due, assignee, reporter, parent, labels, sprint, summary)
# columns = ["site", "key", "labels", "sprint", "summary", "status", "assignee"]

[[sites]]
name = "my-team"
base_url = "https://my-team.atlassian.net"
# sprint_field = "customfield_10020"   # run: tick --doctor
# board_id = 7                         # default board for sprint moves (M)
# boards = { DEMO = 7, WEB = 12 }      # per-project board overrides
"#;
        fs::write(&path, default).map_err(|e| format!("Cannot write {}: {}", path.display(), e))?;
        println!("Default config created at {}", path.display());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_config_toml(token: &str) -> String {
        format!(
            r#"
email = "a@b.com"
token = "{token}"
max_results = 25
theme = "dracula"

[[sites]]
name = "one"
base_url = "https://one.atlassian.net"
"#
        )
    }

    #[test]
    fn parses_minimal_config() {
        let cfg: Config = toml::from_str(&sample_config_toml("secret")).unwrap();
        let mut cfg = cfg;
        cfg.view_jql = Config::build_view_jql(&cfg.views);
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn custom_view_jql() {
        let raw = r#"
email = "a@b.com"
token = "secret"

[views]
assigned = "project = DEMO ORDER BY created DESC"

[[sites]]
name = "one"
base_url = "https://one.atlassian.net"
"#;
        let mut cfg: Config = toml::from_str(raw).unwrap();
        cfg.view_jql = Config::build_view_jql(&cfg.views);
        assert!(cfg.jql_for(ViewMode::MyIssues).contains("DEMO"));
    }

    #[test]
    fn rejects_empty_sites() {
        let raw = r#"
email = "a@b.com"
token = "secret"
sites = []
"#;
        let mut cfg: Config = toml::from_str(raw).unwrap();
        cfg.view_jql = Config::build_view_jql(&cfg.views);
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn parses_board_config() {
        let raw = r#"
email = "a@b.com"
token = "secret"

[[sites]]
name = "one"
base_url = "https://one.atlassian.net"
board_id = 7
boards = { DEMO = 12, WEB = 15 }
"#;
        let mut cfg: Config = toml::from_str(raw).unwrap();
        cfg.view_jql = Config::build_view_jql(&cfg.views);
        let site = &cfg.sites[0];
        assert_eq!(site.board_id, Some(7));
        assert_eq!(site.boards.get("DEMO"), Some(&12));
        assert_eq!(site.board_config().project_boards.get("WEB"), Some(&15));
    }

    #[test]
    fn parses_notify_on_refresh() {
        let raw = r#"
email = "a@b.com"
token = "secret"
notify_on_refresh = true

[[sites]]
name = "one"
base_url = "https://one.atlassian.net"
"#;
        let mut cfg: Config = toml::from_str(raw).unwrap();
        cfg.view_jql = Config::build_view_jql(&cfg.views);
        assert!(cfg.notify_on_refresh);
    }

    #[test]
    fn parses_page_size() {
        let raw = r#"
email = "a@b.com"
token = "secret"
page_size = 25

[[sites]]
name = "one"
base_url = "https://one.atlassian.net"
"#;
        let mut cfg: Config = toml::from_str(raw).unwrap();
        cfg.view_jql = Config::build_view_jql(&cfg.views);
        assert_eq!(cfg.page_size, 25);
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn rejects_invalid_page_size() {
        let raw = r#"
email = "a@b.com"
token = "secret"
page_size = 0

[[sites]]
name = "one"
base_url = "https://one.atlassian.net"
"#;
        let mut cfg: Config = toml::from_str(raw).unwrap();
        cfg.view_jql = Config::build_view_jql(&cfg.views);
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn cli_overrides_validate() {
        let mut cfg: Config = toml::from_str(&sample_config_toml("secret")).unwrap();
        cfg.view_jql = Config::build_view_jql(&cfg.views);
        assert!(cfg.apply_cli_overrides(Some(100), Some(15)).is_ok());
        assert_eq!(cfg.max_results, 100);
        assert_eq!(cfg.page_size, 15);
        assert!(cfg.apply_cli_overrides(None, Some(0)).is_err());
    }

    #[test]
    fn token_from_env() {
        std::env::set_var("TICK_TOKEN", "from-env");
        let t = Config::resolve_token("").unwrap();
        assert_eq!(t, "from-env");
        std::env::remove_var("TICK_TOKEN");
    }
}
