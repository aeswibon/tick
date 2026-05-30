//! Assignable-user catalog per issue (cached, filtered locally).

pub(crate) const CATALOG_MAX: &str = "100";
/// Upper bound on cached users per issue after merges.
pub const CACHE_MAX: usize = 500;

pub fn cache_key(base_url: &str, issue_key: &str) -> String {
    format!("{}|{}", base_url.trim_end_matches('/'), issue_key)
}

/// Case-insensitive match on display name (and account id prefix).
pub fn filter_users(catalog: &[(String, String)], query: &str) -> Vec<(String, String)> {
    let q = query.trim().to_lowercase();
    let mut out: Vec<_> = if q.is_empty() {
        catalog.to_vec()
    } else {
        catalog
            .iter()
            .filter(|(id, name)| name.to_lowercase().contains(&q) || id.to_lowercase().contains(&q))
            .cloned()
            .collect()
    };
    out.sort_by(|a, b| a.1.cmp(&b.1));
    out.truncate(50);
    out
}

/// Union by account id; refresh appends rather than replacing the catalog.
pub fn merge_users(
    existing: &[(String, String)],
    fetched: &[(String, String)],
) -> Vec<(String, String)> {
    use std::collections::HashMap;
    let mut by_id: HashMap<String, String> = HashMap::new();
    for (id, name) in existing {
        by_id.insert(id.clone(), name.clone());
    }
    for (id, name) in fetched {
        by_id.insert(id.clone(), name.clone());
    }
    let mut out: Vec<_> = by_id.into_iter().collect();
    out.sort_by_key(|a| a.1.to_lowercase());
    out.truncate(CACHE_MAX);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_matches_display_name() {
        let catalog = vec![
            ("1".into(), "Alice Smith".into()),
            ("2".into(), "Bob Jones".into()),
        ];
        let out = filter_users(&catalog, "ali");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].1, "Alice Smith");
    }

    #[test]
    fn empty_query_returns_sorted_subset() {
        let catalog = vec![("2".into(), "Zed".into()), ("1".into(), "Amy".into())];
        let out = filter_users(&catalog, "");
        assert_eq!(out[0].1, "Amy");
    }

    #[test]
    fn merge_appends_without_duplicates() {
        let a = vec![("1".into(), "Alice".into())];
        let b = vec![("1".into(), "Alice".into()), ("2".into(), "Bob".into())];
        let merged = merge_users(&a, &b);
        assert_eq!(merged.len(), 2);
    }

    #[test]
    fn merge_keeps_existing_when_fetch_empty() {
        let a = vec![("1".into(), "Alice".into())];
        let merged = merge_users(&a, &[]);
        assert_eq!(merged.len(), 1);
    }

    #[test]
    fn cache_key_normalizes_trailing_slashes() {
        assert_eq!(
            cache_key("https://acme.atlassian.net///", "DEMO-1"),
            "https://acme.atlassian.net|DEMO-1"
        );
    }

    #[test]
    fn filter_matches_account_id_case_insensitively() {
        let catalog = vec![
            ("Account-ABC".into(), "Zed".into()),
            ("account-def".into(), "Amy".into()),
        ];
        let out = filter_users(&catalog, "abc");
        assert_eq!(out, vec![("Account-ABC".into(), "Zed".into())]);
    }

    #[test]
    fn filter_limits_to_fifty_sorted_results() {
        let catalog: Vec<_> = (0..75)
            .rev()
            .map(|n| (format!("id-{n:02}"), format!("User {n:02}")))
            .collect();
        let out = filter_users(&catalog, "user");
        assert_eq!(out.len(), 50);
        assert_eq!(out[0].1, "User 00");
        assert_eq!(out[49].1, "User 49");
    }

    #[test]
    fn merge_refresh_updates_existing_names_and_sorts_case_insensitively() {
        let existing = vec![("2".into(), "zed".into()), ("1".into(), "Alice Old".into())];
        let fetched = vec![("1".into(), "Alice New".into()), ("3".into(), "bob".into())];
        let merged = merge_users(&existing, &fetched);
        assert_eq!(
            merged,
            vec![
                ("1".into(), "Alice New".into()),
                ("3".into(), "bob".into()),
                ("2".into(), "zed".into()),
            ]
        );
    }
}
