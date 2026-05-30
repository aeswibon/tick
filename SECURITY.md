# Security policy

## Supported versions

| Version | Supported |
| ------- | --------- |
| 0.12.x  | Yes       |
| < 0.12  | No        |

## Reporting a vulnerability

Please **do not** open a public GitHub issue for security-sensitive reports.

Email the maintainer with:

1. A description of the issue and impact
2. Steps to reproduce
3. Affected versions

We aim to acknowledge reports within a few business days and will coordinate a fix and disclosure timeline with you.

## Credentials

`tick` stores your Jira API token in `~/.config/tick/token` or `config.toml`. Treat these files like passwords:

- Restrict file permissions (`chmod 600` on the token file)
- Prefer `TICK_TOKEN` in your shell environment for ephemeral sessions
- Never commit tokens to git

## Local cache

`tick` writes per-view ticket JSON under `~/.config/tick/cache/`. These files can contain issue summaries and metadata. Restrict directory permissions and treat the cache as sensitive on shared machines.
