//! Parse Jira issue keys from pasted text or browse URLs.

/// Extract a normalized issue key (e.g. `PROJ-123`) from arbitrary pasted text.
pub fn parse_issue_key(input: &str) -> Option<String> {
    let input = input.trim();
    if input.is_empty() {
        return None;
    }
    if let Some(key) = key_from_browse_url(input) {
        return normalize_issue_key(&key);
    }
    for token in input.split(|c: char| c.is_whitespace() || c == ',' || c == ';') {
        if let Some(k) = normalize_issue_key(token) {
            return Some(k);
        }
    }
    normalize_issue_key(input)
}

/// Project key prefix from an issue key (`PROJ` from `PROJ-123`).
pub fn project_key_from_issue_key(key: &str) -> &str {
    key.rsplit_once('-').map(|(p, _)| p).unwrap_or(key)
}

/// Lowercase host from `https://team.atlassian.net/...` if present.
pub fn host_from_url(url: &str) -> Option<String> {
    let rest = url
        .trim()
        .strip_prefix("https://")
        .or_else(|| url.trim().strip_prefix("http://"))?;
    let host = rest.split('/').next()?.trim();
    if host.is_empty() {
        None
    } else {
        Some(host.to_lowercase())
    }
}

pub fn normalize_issue_key(s: &str) -> Option<String> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let upper = s.to_uppercase();
    let dash = upper.rfind('-')?;
    if dash == 0 || dash + 1 >= upper.len() {
        return None;
    }
    let project = &upper[..dash];
    let num = &upper[dash + 1..];
    if !project
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return None;
    }
    if !num.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(upper)
}

fn key_from_browse_url(s: &str) -> Option<String> {
    let lower = s.to_lowercase();
    let idx = lower.find("/browse/")?;
    let rest = &s[idx + 8..];
    let key = rest.split(['/', '?', '#']).next()?.trim();
    if key.is_empty() {
        None
    } else {
        Some(key.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_key() {
        assert_eq!(parse_issue_key("proj-42").as_deref(), Some("PROJ-42"));
    }

    #[test]
    fn parses_browse_url() {
        assert_eq!(
            parse_issue_key("https://acme.atlassian.net/browse/DEMO-7?focusedCommentId=1")
                .as_deref(),
            Some("DEMO-7")
        );
    }

    #[test]
    fn rejects_invalid() {
        assert!(parse_issue_key("not-a-key").is_none());
        assert!(parse_issue_key("PROJ-").is_none());
    }

    #[test]
    fn parses_first_key_from_mixed_text() {
        assert_eq!(
            parse_issue_key("please check abc-12, then XYZ-99").as_deref(),
            Some("ABC-12")
        );
    }

    #[test]
    fn accepts_underscores_and_uses_last_dash() {
        assert_eq!(
            normalize_issue_key("team_api-42").as_deref(),
            Some("TEAM_API-42")
        );
        assert_eq!(project_key_from_issue_key("TEAM-API-42"), "TEAM-API");
    }

    #[test]
    fn rejects_bad_project_or_number_segments() {
        assert!(normalize_issue_key("TEAM.API-42").is_none());
        assert!(normalize_issue_key("TEAM-42A").is_none());
        assert!(normalize_issue_key("-42").is_none());
        assert!(normalize_issue_key("TEAM-").is_none());
    }

    #[test]
    fn parses_browse_url_key_before_path_query_or_fragment() {
        assert_eq!(
            parse_issue_key("https://acme.atlassian.net/browse/demo-7/attachments").as_deref(),
            Some("DEMO-7")
        );
        assert_eq!(
            parse_issue_key("https://acme.atlassian.net/browse/demo-8#activity").as_deref(),
            Some("DEMO-8")
        );
    }

    #[test]
    fn host_from_jira_url() {
        assert_eq!(
            host_from_url("https://Acme.atlassian.net/browse/X-1").as_deref(),
            Some("acme.atlassian.net")
        );
    }

    #[test]
    fn host_from_url_trims_and_rejects_non_urls() {
        assert_eq!(
            host_from_url("  http://Example.COM/path  ").as_deref(),
            Some("example.com")
        );
        assert!(host_from_url("example.com/browse/X-1").is_none());
        assert!(host_from_url("https:///browse/X-1").is_none());
    }

    mod proptest_issue_key {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn normalize_round_trips_valid_keys(project in "[A-Z][A-Z0-9_]{0,9}", num in 1u32..99999) {
                let key = format!("{project}-{num}");
                let normalized = normalize_issue_key(&key).expect("valid key");
                prop_assert!(normalized.contains('-'));
                prop_assert!(parse_issue_key(&normalized).as_deref() == Some(normalized.as_str()));
            }

            #[test]
            fn parse_never_panics(input in "\\PC*") {
                let _ = parse_issue_key(&input);
            }
        }
    }
}
