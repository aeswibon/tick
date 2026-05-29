# OAuth authentication

tick supports **API tokens** (default) and **Atlassian OAuth 2.0 (3LO)** for organizations that require OAuth instead of personal access tokens.

## When to use OAuth

- Your admin requires OAuth apps instead of API tokens
- You want refreshable sessions without storing a long-lived API token in a file
- You are standardizing on Atlassian Cloud OAuth across tools

API tokens remain the simplest path for personal use. See [CONFIGURATION.md](CONFIGURATION.md#credentials).

## 1. Create an Atlassian OAuth app

1. Go to [Atlassian Developer Console](https://developer.atlassian.com/console/myapps/)
2. **Create** → **OAuth 2.0 integration**
3. Set **Callback URL** to `http://127.0.0.1:8765/callback` (or your custom `redirect_uri`)
4. Add permissions (scopes) — tick requests:
   - `read:jira-work`
   - `write:jira-work`
   - `read:jira-user`
   - `offline_access`
   - `manage:jira-configuration`
5. Copy **Client ID** and **Client secret**

## 2. Configure tick

In `~/.config/tick/config.toml`:

```toml
email = "you@example.com"
auth = "oauth"

[oauth]
client_id = "YOUR_CLIENT_ID"
redirect_uri = "http://127.0.0.1:8765/callback"
```

**Never** put the client secret in config. Use environment variables:

```bash
export TICK_OAUTH_CLIENT_ID="YOUR_CLIENT_ID"      # optional if set in config
export TICK_OAUTH_CLIENT_SECRET="YOUR_SECRET"    # required
```

Optional overrides:

```bash
export TICK_OAUTH_REDIRECT_URI="http://127.0.0.1:8765/callback"
```

## 3. Log in

```bash
tick auth login
```

This opens your browser, starts a local callback server on port **8765**, and saves tokens to:

`~/.config/tick/oauth.json` (mode `600` on Unix)

After login, tick prints accessible Jira Cloud sites. Use each site URL as `base_url` under `[[sites]]`.

## 4. Run tick

```bash
tick
```

Tokens refresh automatically when expired (requires `TICK_OAUTH_CLIENT_SECRET`).

## Auth commands

| Command | Description |
|---------|-------------|
| `tick auth login` | Browser OAuth flow |
| `tick auth status` | Show **API token** login (per site) and **OAuth** session if present |
| `tick auth logout` | Delete `oauth.json` |

With the default API token setup, run `tick auth status` to confirm the token works and which Jira sites accept it — no OAuth login required.

## Troubleshooting

| Problem | Fix |
|---------|-----|
| `Cannot bind callback on 127.0.0.1:8765` | Another process uses port 8765; change `redirect_uri` in app + config |
| `OAuth state mismatch` | Retry login; do not interrupt the callback |
| `client_secret` errors | Export `TICK_OAUTH_CLIENT_SECRET` before `tick` or `tick auth login` |
| 401 on API calls | Run `tick auth login` again; verify `auth = "oauth"` in config |
| Site not listed after login | Add `[[sites]]` manually with URL from Atlassian admin |

## Switching back to API token

1. `tick auth logout`
2. Set `auth = "token"` (or remove `auth` line)
3. Configure `TICK_TOKEN` or `~/.config/tick/token`
