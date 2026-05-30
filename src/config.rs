use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::api::agile::BoardConfig;
use crate::auth::Auth;
use crate::view_mode::ViewMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AuthMethod {
    #[default]
    Token,
    Oauth,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct OAuthSettings {
    #[serde(default)]
    pub client_id: String,
    #[serde(default = "default_oauth_redirect")]
    pub redirect_uri: String,
}

fn default_oauth_redirect() -> String {
    "http://127.0.0.1:8765/callback".to_string()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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
    /// Default project key for `n` (new issue).
    #[serde(default)]
    pub create_project: Option<String>,
    /// Default issue type name for `n`.
    #[serde(default)]
    pub create_issue_type: Option<String>,
    /// After duplicate, link new issue to source (`Cloners` by default).
    #[serde(default = "default_true")]
    pub create_clone_link: bool,
    /// Issue link type name for duplicate.
    #[serde(default = "default_clone_link_type")]
    pub clone_link_type: String,
    /// Issue link type names for add-link (`I`); see `SiteLinkTypes`.
    #[serde(default)]
    pub link_types: SiteLinkTypes,
}

fn default_true() -> bool {
    true
}

fn default_clone_link_type() -> String {
    "Cloners".into()
}

fn default_link_relates() -> String {
    crate::api::issue_relations::DEFAULT_LINK_RELATES.into()
}

fn default_link_blocks() -> String {
    crate::api::issue_relations::DEFAULT_LINK_BLOCKS.into()
}

fn default_link_epic() -> String {
    crate::api::issue_relations::DEFAULT_LINK_EPIC.into()
}

/// Jira issue link type names for the add-link picker (`I`).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SiteLinkTypes {
    #[serde(default = "default_link_relates")]
    pub relates: String,
    #[serde(default = "default_link_blocks")]
    pub blocks: String,
    /// Often the same Jira type as `blocks`; picker uses inverted inward/outward.
    #[serde(default = "default_link_blocks")]
    pub blocked_by: String,
    #[serde(default = "default_link_epic")]
    pub epic: String,
}

impl Default for SiteLinkTypes {
    fn default() -> Self {
        Self {
            relates: default_link_relates(),
            blocks: default_link_blocks(),
            blocked_by: default_link_blocks(),
            epic: default_link_epic(),
        }
    }
}

impl SiteLinkTypes {
    /// `(Jira type name, picker label)` — four entries, fixed order.
    pub fn picker_options(&self) -> Vec<(String, String)> {
        vec![
            (
                self.relates.clone(),
                crate::api::issue_relations::ADD_LINK_LABELS[0].into(),
            ),
            (
                self.blocks.clone(),
                crate::api::issue_relations::ADD_LINK_LABELS[1].into(),
            ),
            (
                self.blocked_by.clone(),
                crate::api::issue_relations::ADD_LINK_LABELS[2].into(),
            ),
            (
                self.epic.clone(),
                crate::api::issue_relations::ADD_LINK_LABELS[3].into(),
            ),
        ]
    }
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

    pub fn clone_link_enabled(&self) -> bool {
        self.create_clone_link
    }

    pub fn clone_link_type_name(&self) -> &str {
        if self.clone_link_type.is_empty() {
            "Cloners"
        } else {
            &self.clone_link_type
        }
    }
}

/// Saved JQL view — switch with configured key (`7`–`9`) or `v` / `Shift+V`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CustomView {
    pub name: String,
    pub jql: String,
    /// Query only this site (must match `[[sites]].name`).
    #[serde(default)]
    pub site: Option<String>,
    /// Tab key `7`, `8`, or `9` (auto-assigned to 7, 8, 9 for first entries).
    #[serde(default)]
    pub key: Option<u8>,
}

impl CustomView {
    pub fn cache_slug(&self) -> String {
        self.name
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() {
                    c.to_ascii_lowercase()
                } else {
                    '-'
                }
            })
            .collect::<String>()
            .trim_matches('-')
            .to_string()
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ViewQueries {
    pub assigned: Option<String>,
    pub updated: Option<String>,
    pub mentions: Option<String>,
    pub watched: Option<String>,
    pub sprint: Option<String>,
    /// Closed search: assignee when done (base JQL without `text ~` or ORDER BY).
    pub closed: Option<String>,
    /// Closed search: ever assigned to you (`assignee was`).
    pub closed_history: Option<String>,
    #[serde(default)]
    pub custom: Vec<CustomView>,
}

/// Pre-filled issue for `N` (create from template). Minimal edits: summary, then Enter.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct IssueTemplate {
    /// Short id shown in the template picker (unique in config).
    pub name: String,
    /// Limit template to one site; required when you have multiple `[[sites]]`.
    pub site: Option<String>,
    pub project: String,
    #[serde(rename = "issue_type")]
    pub issue_type: String,
    pub summary: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub labels: Vec<String>,
    pub priority: Option<String>,
    /// Jira account id (not display name).
    pub assignee_account_id: Option<String>,
    pub parent_key: Option<String>,
    /// Extra create fields by id, e.g. `customfield_10001 = "value"`.
    #[serde(default)]
    pub extra_fields: HashMap<String, toml::Value>,
}

impl IssueTemplate {
    pub fn validate_fields(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("Template name cannot be empty".into());
        }
        if self.project.trim().is_empty() {
            return Err("Template needs a project".into());
        }
        if self.issue_type.trim().is_empty() {
            return Err("Template needs an issue type".into());
        }
        if self.summary.trim().is_empty() {
            return Err("Template needs a summary".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct CreateSettings {
    #[serde(default = "default_clone_summary_prefix")]
    pub clone_summary_prefix: String,
    /// Optional file under the config dir (e.g. `templates/local.toml`) merged at load.
    #[serde(default)]
    pub templates_file: Option<String>,
    #[serde(default)]
    pub templates: Vec<IssueTemplate>,
}

impl Config {
    /// Templates visible for a site (or all when `site_name` is None).
    pub fn issue_templates_for_site<'a>(
        &'a self,
        site_name: Option<&str>,
    ) -> Vec<&'a IssueTemplate> {
        self.create
            .templates
            .iter()
            .filter(|t| template_matches_site(t, site_name))
            .collect()
    }
}

fn template_matches_site(template: &IssueTemplate, site_name: Option<&str>) -> bool {
    match (&template.site, site_name) {
        (None, _) => true,
        (Some(tmpl_site), Some(active)) => tmpl_site == active,
        (Some(_), None) => true,
    }
}

fn default_clone_summary_prefix() -> String {
    "Copy of: ".into()
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
    #[serde(default)]
    pub auth: AuthMethod,
    #[serde(default)]
    pub oauth: OAuthSettings,
    #[serde(default)]
    pub create: CreateSettings,
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
        if config.auth == AuthMethod::Token {
            config.token = Self::resolve_token(&config.token)?;
        }
        config.view_jql = Self::build_view_jql(&config.views);
        crate::template_export::merge_templates_file(&mut config)?;
        config.validate()?;
        Ok(config)
    }

    pub async fn resolve_auth(&self) -> Result<Auth, String> {
        match self.auth {
            AuthMethod::Token => {
                let token = Self::resolve_token(&self.token)?;
                Ok(Auth::basic(&self.email, &token))
            }
            AuthMethod::Oauth => crate::oauth::load_auth(&self.oauth).await,
        }
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
                    ViewMode::Mentions => views.mentions.as_deref(),
                    ViewMode::Watching => views.watched.as_deref(),
                    ViewMode::Updated => views.updated.as_deref(),
                    ViewMode::Sprint => views.sprint.as_deref(),
                    ViewMode::ClosedSearch => views.closed.as_deref(),
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

    /// Base JQL for the Closed tab (without search text).
    pub fn closed_search_base(&self, ever_assigned: bool) -> &str {
        if ever_assigned {
            self.views
                .closed_history
                .as_deref()
                .unwrap_or(ViewMode::closed_search_base(true))
        } else {
            self.views
                .closed
                .as_deref()
                .unwrap_or(ViewMode::closed_search_base(false))
        }
    }

    pub fn build_closed_search_jql(&self, query: &str, ever_assigned: bool) -> String {
        let base = self.closed_search_base(ever_assigned);
        crate::view_mode::build_closed_search_jql(base, query)
    }

    /// Sites to query for a fetch (`site_filter` = `[[sites]].name`).
    pub fn sites_for_fetch(&self, site_filter: Option<&str>) -> Vec<&Site> {
        match site_filter {
            Some(name) => self.sites.iter().filter(|s| s.name == name).collect(),
            None => self.sites.iter().collect(),
        }
    }

    /// Resolve tab keys for custom views (defaults 7, 8, 9).
    pub fn custom_view_keys(&self) -> Vec<(u8, usize)> {
        let mut used = std::collections::HashSet::new();
        let mut out = Vec::new();
        for (i, view) in self.views.custom.iter().enumerate() {
            let key = view.key.unwrap_or(match i {
                0 => 7,
                1 => 8,
                _ => 9,
            });
            if (7..=9).contains(&key) && used.insert(key) {
                out.push((key, i));
            }
        }
        out
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
        if self.auth == AuthMethod::Oauth {
            let path = crate::oauth::token_path()?;
            if !path.exists() {
                return Err("OAuth not configured: run `tick auth login`".into());
            }
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
        let mut template_names = std::collections::HashSet::new();
        for template in &self.create.templates {
            let name = template.name.trim();
            if name.is_empty() {
                return Err("config: create.templates name must not be empty".into());
            }
            if !template_names.insert(name.to_string()) {
                return Err(format!("config: duplicate template name '{name}'"));
            }
            if template.project.trim().is_empty() {
                return Err(format!("config: template '{name}' needs project"));
            }
            if template.issue_type.trim().is_empty() {
                return Err(format!("config: template '{name}' needs issue_type"));
            }
            if template.summary.trim().is_empty() {
                return Err(format!("config: template '{name}' needs summary"));
            }
            if let Some(ref site) = template.site {
                if !self.sites.iter().any(|s| s.name == *site) {
                    return Err(format!(
                        "config: template '{name}' references unknown site '{site}'"
                    ));
                }
            } else if self.sites.len() > 1 {
                return Err(format!(
                    "config: template '{name}' needs site = \"...\" when using multiple [[sites]]"
                ));
            }
        }
        let mut custom_names = std::collections::HashSet::new();
        let mut custom_keys = std::collections::HashSet::new();
        for view in &self.views.custom {
            let name = view.name.trim();
            if name.is_empty() {
                return Err("config: views.custom name must not be empty".into());
            }
            if !custom_names.insert(name.to_string()) {
                return Err(format!("config: duplicate custom view '{name}'"));
            }
            if view.jql.trim().is_empty() {
                return Err(format!("config: custom view '{name}' needs jql"));
            }
            if let Some(ref site) = view.site {
                if !self.sites.iter().any(|s| s.name == *site) {
                    return Err(format!(
                        "config: custom view '{name}' references unknown site '{site}'"
                    ));
                }
            }
            if let Some(key) = view.key {
                if !(7..=9).contains(&key) {
                    return Err(format!(
                        "config: custom view '{name}' key must be 7, 8, or 9 (got {key})"
                    ));
                }
                if !custom_keys.insert(key) {
                    return Err(format!("config: duplicate custom view key {key}"));
                }
            }
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
# API token auth is the default (no auth line needed).
# token = "your-api-token"   # or use ~/.config/tick/token or TICK_TOKEN env
max_results = 50
page_size = 10   # rows to scroll with [ and ]
theme = "default"
# notify_on_refresh = true   # desktop alert when new issues appear on refresh
# auth = "oauth"             # optional; default is token — see docs/OAUTH.md
# [oauth]
# client_id = "your-atlassian-oauth-app-id"
# redirect_uri = "http://127.0.0.1:8765/callback"

# Optional custom JQL per view (defaults shown commented)
# [views]
# assigned = "assignee = currentUser() AND statusCategory != Done ORDER BY updated DESC"
# mentions = "comment ~ currentUser() AND statusCategory != Done ORDER BY updated DESC"
# watched = "watcher = currentUser() AND statusCategory != Done ORDER BY updated DESC"
# updated = "assignee = currentUser() AND statusCategory != Done AND updated >= -7d ORDER BY updated DESC"
# sprint = "sprint in openSprints() AND assignee = currentUser() ORDER BY updated DESC"
# closed = "assignee = currentUser() AND statusCategory = Done"   # 6th tab — add text via /
# closed_history = "assignee was currentUser() AND statusCategory = Done"   # h toggles on Closed tab
#
# [[views.custom]]
# name = "My bugs"
# jql = "project = DEMO AND assignee = currentUser() ORDER BY updated DESC"
# key = 7   # tab key 7, 8, or 9

# Optional table columns (ids: site, key, type, status, priority, age, due, assignee, reporter, parent, labels, sprint, summary)
# columns = ["site", "key", "labels", "sprint", "summary", "status", "assignee"]

[[sites]]
name = "my-team"
base_url = "https://my-team.atlassian.net"
# sprint_field = "customfield_10020"   # run: tick --doctor
# board_id = 7                         # default board for sprint moves (M)
# boards = { DEMO = 7, WEB = 12 }      # per-project board overrides
# create_project = "ENG"
# create_issue_type = "Task"

# Issue templates — press N to create; X on a ticket exports it as a template
# Optional: templates_file = "templates/local.toml"  # merged at load
# [[create.templates]]
# name = "bug"
# site = "my-team"                    # required if you have multiple [[sites]]
# project = "ENG"
# issue_type = "Bug"
# summary = "Bug: "
# description = "Use triple-quotes in your real config for multiline text"
# labels = ["bug"]
# priority = "Medium"
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
    fn auth_defaults_to_token_when_omitted() {
        let toml = r#"
email = "a@b.com"
token = "secret"

[[sites]]
name = "s"
base_url = "https://x.atlassian.net"
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.auth, AuthMethod::Token);
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
sprint = "sprint in openSprints() ORDER BY rank"

[[sites]]
name = "one"
base_url = "https://one.atlassian.net"
"#;
        let mut cfg: Config = toml::from_str(raw).unwrap();
        cfg.view_jql = Config::build_view_jql(&cfg.views);
        assert!(cfg.jql_for(ViewMode::MyIssues).contains("DEMO"));
        assert!(cfg.jql_for(ViewMode::Sprint).contains("openSprints"));
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

    #[test]
    fn parses_issue_templates() {
        let toml = r#"
email = "a@b.com"
token = "t"

[[sites]]
name = "one"
base_url = "https://one.atlassian.net"

[[create.templates]]
name = "bug"
project = "ONE"
issue_type = "Bug"
summary = "Bug: "
labels = ["a"]
"#;
        let mut cfg: Config = toml::from_str(toml).unwrap();
        cfg.view_jql = Config::build_view_jql(&cfg.views);
        assert!(cfg.validate().is_ok());
        assert_eq!(cfg.create.templates.len(), 1);
        assert_eq!(cfg.issue_templates_for_site(Some("one")).len(), 1);
    }

    #[test]
    fn multi_site_template_requires_site_field() {
        let toml = r#"
email = "a@b.com"
token = "t"

[[sites]]
name = "one"
base_url = "https://one.atlassian.net"

[[sites]]
name = "two"
base_url = "https://two.atlassian.net"

[[create.templates]]
name = "x"
project = "P"
issue_type = "Task"
summary = "S"
"#;
        let mut cfg: Config = toml::from_str(toml).unwrap();
        cfg.view_jql = Config::build_view_jql(&cfg.views);
        assert!(cfg.validate().is_err());
    }
}
