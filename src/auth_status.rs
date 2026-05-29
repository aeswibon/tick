//! `tick auth status` — reports API token and OAuth session state.

use crate::api::JiraClient;
use crate::config::{AuthMethod, Config};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenSource {
    Env,
    File,
    Config,
    Missing,
}

impl TokenSource {
    pub fn label(self) -> &'static str {
        match self {
            Self::Env => "TICK_TOKEN environment variable",
            Self::File => "~/.config/tick/token file",
            Self::Config => "config.toml",
            Self::Missing => "not configured",
        }
    }
}

/// Where an API token would be loaded from (does not expose the secret).
pub fn token_source(config_token: &str) -> (TokenSource, bool) {
    if let Ok(t) = std::env::var("TICK_TOKEN") {
        if !t.trim().is_empty() {
            return (TokenSource::Env, true);
        }
    }
    if let Ok(path) = Config::token_path() {
        if path.is_file() {
            if let Ok(t) = std::fs::read_to_string(&path) {
                if !t.trim().is_empty() {
                    return (TokenSource::File, true);
                }
            }
        }
    }
    let t = config_token.trim();
    if !t.is_empty() && t != "your-api-token" {
        return (TokenSource::Config, true);
    }
    (TokenSource::Missing, false)
}

/// Parse config for status/doctor without requiring a valid API token upfront.
pub fn load_config_for_status() -> Result<Config, String> {
    let path = Config::config_path()?;
    let contents = std::fs::read_to_string(&path)
        .map_err(|e| format!("Cannot read {}: {}", path.display(), e))?;
    let mut config: Config =
        toml::from_str(&contents).map_err(|e| format!("Invalid config: {}", e))?;
    config.view_jql = Config::build_view_jql(&config.views);
    if config.email.trim().is_empty() {
        return Err("config: email must not be empty".into());
    }
    if config.sites.is_empty() {
        return Err("config: at least one [[sites]] entry is required".into());
    }
    Ok(config)
}

pub async fn print_status() -> Result<(), String> {
    let config = match load_config_for_status() {
        Ok(c) => c,
        Err(e) => {
            println!("Config: {e}");
            println!("Run: tick --init");
            return Ok(());
        }
    };

    let active = match config.auth {
        AuthMethod::Token => "token (API token — default)",
        AuthMethod::Oauth => "oauth",
    };
    println!("Active auth: {active}");
    println!("Config email: {}", config.email);
    println!();

    print_token_section(&config).await;
    println!();
    print_oauth_section(&config).await;

    Ok(())
}

async fn print_token_section(config: &Config) {
    println!("API token:");
    let (source, present) = token_source(&config.token);
    println!("  Source: {}", source.label());

    if config.auth == AuthMethod::Oauth {
        if present {
            println!("  Note: token is configured but auth = \"oauth\" — token is not used");
        } else {
            println!("  Note: not used while auth = \"oauth\"");
        }
        return;
    }

    if !present {
        println!("  Status: not logged in");
        println!("  Set TICK_TOKEN, ~/.config/tick/token, or token in config.toml");
        return;
    }

    let token = match Config::resolve_token(&config.token) {
        Ok(t) => t,
        Err(e) => {
            println!("  Status: not logged in — {e}");
            return;
        }
    };

    let client = JiraClient::new(&config.email, &token, false);
    let mut any_ok = false;
    for site in &config.sites {
        match verify_site(&client, &site.base_url).await {
            Ok(profile) => {
                any_ok = true;
                println!(
                    "  {} ({}): OK — {} ({})",
                    site.name, site.base_url, profile.display_name, profile.account_id
                );
            }
            Err(e) => {
                println!("  {} ({}): FAILED — {e}", site.name, site.base_url);
            }
        }
    }
    if any_ok {
        println!("  Status: logged in (API token valid)");
    } else {
        println!("  Status: token present but Jira rejected it on all configured sites");
    }
}

struct UserProfile {
    account_id: String,
    display_name: String,
}

async fn verify_site(client: &JiraClient, base_url: &str) -> Result<UserProfile, String> {
    let base = base_url.trim_end_matches('/');
    let url = format!("{base}/rest/api/3/myself");
    let resp = client.send(|| client.get(&url).send()).await?;
    if !resp.status().is_success() {
        return Err(format!(
            "HTTP {} — check email/token and that this is a Jira Cloud URL",
            resp.status()
        ));
    }
    let body: serde_json::Value = resp.json().await.map_err(|e| format!("parse error: {e}"))?;
    let account_id = body
        .get("accountId")
        .and_then(|v| v.as_str())
        .ok_or("response missing accountId")?
        .to_string();
    let display_name = body
        .get("displayName")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();
    Ok(UserProfile {
        account_id,
        display_name,
    })
}

async fn print_oauth_section(config: &Config) {
    println!("OAuth:");
    let path = match crate::oauth::token_path() {
        Ok(p) => p,
        Err(e) => {
            println!("  Error: {e}");
            return;
        }
    };

    if !path.exists() {
        println!("  Session: not logged in");
        if config.auth == AuthMethod::Oauth {
            println!("  Run: tick auth login");
        } else {
            println!("  Optional — run `tick auth login` and set auth = \"oauth\" to use OAuth");
        }
        return;
    }

    match crate::oauth::load_tokens() {
        Ok(tokens) => {
            if config.auth == AuthMethod::Token {
                println!("  Session: on disk (not active — auth = \"token\")");
            } else {
                println!("  Session: logged in");
            }
            println!("  Token file: {}", path.display());
            if !tokens.email.is_empty() {
                println!("  Account: {}", tokens.email);
            }
            println!(
                "  Expires: {} ({})",
                tokens.expires_at,
                if tokens.is_expired() {
                    "expired — will refresh on next run"
                } else {
                    "valid"
                }
            );
            crate::oauth::print_accessible_resources(&tokens.access_token).await;
        }
        Err(e) => {
            println!("  Session: invalid ({e})");
            println!("  Run: tick auth login");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_source_missing_when_empty() {
        let (src, ok) = token_source("");
        assert_eq!(src, TokenSource::Missing);
        assert!(!ok);
    }

    #[test]
    fn token_source_from_config_toml() {
        let (src, ok) = token_source("secret-token");
        assert_eq!(src, TokenSource::Config);
        assert!(ok);
    }
}
