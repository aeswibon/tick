//! Atlassian OAuth 2.0 (3LO) for Jira Cloud.
//!
//! Create an OAuth 2.0 integration at https://developer.atlassian.com/console/myapps/
//! Callback URL must match `redirect_uri` (default `http://127.0.0.1:8765/callback`).

use crate::auth::Auth;
use crate::config::OAuthSettings;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;

const AUTH_URL: &str = "https://auth.atlassian.com/authorize";
const TOKEN_URL: &str = "https://auth.atlassian.com/oauth/token";
const RESOURCES_URL: &str = "https://api.atlassian.com/oauth/token/accessible-resources";
const SCOPES: &str =
    "read:jira-work write:jira-work read:jira-user offline_access manage:jira-configuration";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: DateTime<Utc>,
    pub email: String,
}

impl OAuthTokens {
    pub fn is_expired(&self) -> bool {
        Utc::now() + Duration::minutes(2) >= self.expires_at
    }
}

pub fn token_path() -> Result<PathBuf, String> {
    Ok(crate::config::Config::config_dir()?.join("oauth.json"))
}

pub fn load_tokens() -> Result<OAuthTokens, String> {
    let path = token_path()?;
    let raw = fs::read_to_string(&path).map_err(|_| {
        format!(
            "No OAuth session at {}. Run: tick auth login",
            path.display()
        )
    })?;
    serde_json::from_str(&raw).map_err(|e| format!("Invalid {}: {e}", path.display()))
}

pub fn save_tokens(tokens: &OAuthTokens) -> Result<(), String> {
    let path = token_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Cannot create {}: {e}", parent.display()))?;
    }
    let raw = serde_json::to_string_pretty(tokens)
        .map_err(|e| format!("Cannot serialize OAuth tokens: {e}"))?;
    fs::write(&path, raw).map_err(|e| format!("Cannot write {}: {e}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

pub fn delete_tokens() -> Result<(), String> {
    let path = token_path()?;
    if path.exists() {
        fs::remove_file(&path).map_err(|e| format!("Cannot remove {}: {e}", path.display()))?;
    }
    Ok(())
}

pub fn resolve_client_id(settings: &OAuthSettings) -> Result<String, String> {
    if let Ok(id) = std::env::var("TICK_OAUTH_CLIENT_ID") {
        let id = id.trim();
        if !id.is_empty() {
            return Ok(id.to_string());
        }
    }
    let id = settings.client_id.trim();
    if id.is_empty() {
        return Err(
            "OAuth client_id missing: set TICK_OAUTH_CLIENT_ID or oauth.client_id in config.toml"
                .into(),
        );
    }
    Ok(id.to_string())
}

fn resolve_client_secret() -> Result<String, String> {
    std::env::var("TICK_OAUTH_CLIENT_SECRET")
        .map(|s| s.trim().to_string())
        .map_err(|_| {
            "TICK_OAUTH_CLIENT_SECRET env var is required for OAuth login and token refresh".into()
        })
        .and_then(|s| {
            if s.is_empty() {
                Err("TICK_OAUTH_CLIENT_SECRET must not be empty".into())
            } else {
                Ok(s)
            }
        })
}

pub fn redirect_uri(settings: &OAuthSettings) -> String {
    if let Ok(uri) = std::env::var("TICK_OAUTH_REDIRECT_URI") {
        let uri = uri.trim();
        if !uri.is_empty() {
            return uri.to_string();
        }
    }
    let uri = settings.redirect_uri.trim();
    if uri.is_empty() {
        "http://127.0.0.1:8765/callback".to_string()
    } else {
        uri.to_string()
    }
}

pub async fn refresh_tokens(
    client_id: &str,
    client_secret: &str,
    refresh_token: &str,
) -> Result<OAuthTokens, String> {
    let http = reqwest::Client::new();
    let resp = http
        .post(TOKEN_URL)
        .json(&serde_json::json!({
            "grant_type": "refresh_token",
            "client_id": client_id,
            "client_secret": client_secret,
            "refresh_token": refresh_token,
        }))
        .send()
        .await
        .map_err(|e| format!("Token refresh HTTP error: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!(
            "Token refresh failed {}: {}",
            resp.status(),
            resp.text().await.unwrap_or_default()
        ));
    }
    let data: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Token refresh parse error: {e}"))?;
    let access = data["access_token"]
        .as_str()
        .ok_or("Token refresh: missing access_token")?
        .to_string();
    let refresh = data["refresh_token"]
        .as_str()
        .unwrap_or(refresh_token)
        .to_string();
    let expires_in = data["expires_in"].as_i64().unwrap_or(3600);
    let mut stored = load_tokens().unwrap_or(OAuthTokens {
        access_token: access.clone(),
        refresh_token: refresh.clone(),
        expires_at: Utc::now(),
        email: String::new(),
    });
    stored.access_token = access;
    stored.refresh_token = refresh;
    stored.expires_at = Utc::now() + Duration::seconds(expires_in);
    Ok(stored)
}

pub async fn load_auth(settings: &OAuthSettings) -> Result<Auth, String> {
    let client_id = resolve_client_id(settings)?;
    let mut tokens = load_tokens()?;
    if tokens.is_expired() {
        let secret = resolve_client_secret()?;
        tokens = refresh_tokens(&client_id, &secret, &tokens.refresh_token).await?;
        save_tokens(&tokens)?;
    }
    Ok(Auth::bearer(tokens.access_token, tokens.email))
}

pub async fn login(settings: &OAuthSettings) -> Result<(), String> {
    let client_id = resolve_client_id(settings)?;
    let client_secret = resolve_client_secret()?;
    let redirect = redirect_uri(settings);
    let state = format!("tick-{}", std::process::id());

    let auth_url = format!(
        "{AUTH_URL}?audience=api.atlassian.com&client_id={}&scope={}&redirect_uri={}&state={}&response_type=code&prompt=consent",
        url_encode(&client_id),
        url_encode(SCOPES),
        url_encode(&redirect),
        url_encode(&state),
    );

    println!("Opening browser for Atlassian login...");
    println!("If the browser does not open, visit:\n{auth_url}\n");
    let _ = crate::platform::open_url(&auth_url);

    let listener = TcpListener::bind("127.0.0.1:8765")
        .map_err(|e| format!("Cannot bind callback on 127.0.0.1:8765: {e}"))?;
    println!("Waiting for callback on {redirect} ...");

    let (mut stream, _) = listener
        .accept()
        .map_err(|e| format!("Callback accept failed: {e}"))?;
    let mut buf = [0u8; 4096];
    let n = stream
        .read(&mut buf)
        .map_err(|e| format!("Callback read failed: {e}"))?;
    let req = String::from_utf8_lossy(&buf[..n]);
    let path = req
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .ok_or("Invalid callback request")?;

    let code = parse_query_param(path, "code").ok_or("Callback missing code parameter")?;
    let returned_state = parse_query_param(path, "state").unwrap_or_default();
    if returned_state != state {
        return Err("OAuth state mismatch — try again".into());
    }

    let http = reqwest::Client::new();
    let resp = http
        .post(TOKEN_URL)
        .json(&serde_json::json!({
            "grant_type": "authorization_code",
            "client_id": client_id,
            "client_secret": client_secret,
            "code": code,
            "redirect_uri": redirect,
        }))
        .send()
        .await
        .map_err(|e| format!("Token exchange HTTP error: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!(
            "Token exchange failed {}: {}",
            resp.status(),
            resp.text().await.unwrap_or_default()
        ));
    }
    let data: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Token exchange parse error: {e}"))?;
    let access = data["access_token"]
        .as_str()
        .ok_or("Token response missing access_token")?
        .to_string();
    let refresh = data["refresh_token"]
        .as_str()
        .ok_or("Token response missing refresh_token")?
        .to_string();
    let expires_in = data["expires_in"].as_i64().unwrap_or(3600);

    let email = fetch_profile_email(&access).await.unwrap_or_default();

    let tokens = OAuthTokens {
        access_token: access,
        refresh_token: refresh,
        expires_at: Utc::now() + Duration::seconds(expires_in),
        email: email.clone(),
    };
    save_tokens(&tokens)?;

    let _ = respond_ok(&mut stream);
    println!("OAuth login successful.");
    if !email.is_empty() {
        println!("  Account: {email}");
    }
    print_accessible_resources(&tokens.access_token).await;
    println!("\nNext: set auth = \"oauth\" in ~/.config/tick/config.toml and run tick");
    Ok(())
}

pub async fn print_status() -> Result<(), String> {
    let path = token_path()?;
    if !path.exists() {
        println!("OAuth: not logged in");
        println!("Run: tick auth login");
        return Ok(());
    }
    let tokens = load_tokens()?;
    println!("OAuth: logged in");
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
    print_accessible_resources(&tokens.access_token).await;
    Ok(())
}

async fn print_accessible_resources(access_token: &str) {
    let http = reqwest::Client::new();
    let resp = match http
        .get(RESOURCES_URL)
        .bearer_auth(access_token)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            println!("  Accessible sites: (could not fetch: {e})");
            return;
        }
    };
    if !resp.status().is_success() {
        println!("  Accessible sites: API {}", resp.status());
        return;
    }
    let data: serde_json::Value = match resp.json().await {
        Ok(v) => v,
        Err(_) => return,
    };
    println!("  Accessible Jira sites (use as base_url in config):");
    if let Some(arr) = data.as_array() {
        for site in arr {
            let name = site["name"].as_str().unwrap_or("?");
            let url = site["url"].as_str().unwrap_or("?");
            println!("    {name} — {url}");
        }
    }
}

async fn fetch_profile_email(access_token: &str) -> Result<String, String> {
    let http = reqwest::Client::new();
    let resp = http
        .get("https://api.atlassian.com/me")
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| format!("{e}"))?;
    if !resp.status().is_success() {
        return Ok(String::new());
    }
    let data: serde_json::Value = resp.json().await.map_err(|e| format!("{e}"))?;
    Ok(data["email"]
        .as_str()
        .or_else(|| data["emailAddress"].as_str())
        .unwrap_or_default()
        .to_string())
}

fn respond_ok(stream: &mut std::net::TcpStream) -> std::io::Result<()> {
    let body = "<html><body><h2>tick — login OK</h2><p>You can close this tab.</p></body></html>";
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes())
}

fn parse_query_param(path: &str, key: &str) -> Option<String> {
    let query = path.split('?').nth(1)?;
    for pair in query.split('&') {
        let mut kv = pair.splitn(2, '=');
        if kv.next()? == key {
            return Some(url_decode(kv.next().unwrap_or("")).trim().to_string());
        }
    }
    None
}

fn url_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn url_decode(s: &str) -> String {
    let mut out = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(v) =
                u8::from_str_radix(std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or(""), 16)
            {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}
