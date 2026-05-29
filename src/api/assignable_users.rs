//! Assignable-user catalog per issue (cached, filtered locally).

pub(crate) const CATALOG_MAX: &str = "100";

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
}
