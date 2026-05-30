#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ViewMode {
    MyIssues,
    Mentions,
    Watching,
    Updated,
    Sprint,
    /// Closed/done issues — JQL built from footer search (`/` on this tab).
    ClosedSearch,
}

impl ViewMode {
    pub fn default_jql(&self) -> &'static str {
        match self {
            ViewMode::MyIssues => {
                "assignee = currentUser() AND statusCategory != Done ORDER BY updated DESC"
            }
            ViewMode::Mentions => {
                "comment ~ currentUser() AND statusCategory != Done ORDER BY updated DESC"
            }
            ViewMode::Watching => {
                "watcher = currentUser() AND statusCategory != Done ORDER BY updated DESC"
            }
            ViewMode::Updated => {
                "assignee = currentUser() AND statusCategory != Done AND updated >= -7d ORDER BY updated DESC"
            }
            ViewMode::Sprint => {
                "sprint in openSprints() AND assignee = currentUser() ORDER BY updated DESC"
            }
            ViewMode::ClosedSearch => {
                "assignee = currentUser() AND statusCategory = Done ORDER BY updated DESC"
            }
        }
    }

    /// Base JQL for closed search before the `text ~` clause (no ORDER BY).
    pub fn closed_search_base(ever_assigned: bool) -> &'static str {
        if ever_assigned {
            "assignee was currentUser() AND statusCategory = Done"
        } else {
            "assignee = currentUser() AND statusCategory = Done"
        }
    }

    pub fn next(self) -> Self {
        match self {
            ViewMode::MyIssues => ViewMode::Mentions,
            ViewMode::Mentions => ViewMode::Watching,
            ViewMode::Watching => ViewMode::Updated,
            ViewMode::Updated => ViewMode::Sprint,
            ViewMode::Sprint => ViewMode::ClosedSearch,
            ViewMode::ClosedSearch => ViewMode::MyIssues,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            ViewMode::MyIssues => ViewMode::ClosedSearch,
            ViewMode::Mentions => ViewMode::MyIssues,
            ViewMode::Watching => ViewMode::Mentions,
            ViewMode::Updated => ViewMode::Watching,
            ViewMode::Sprint => ViewMode::Updated,
            ViewMode::ClosedSearch => ViewMode::Sprint,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            ViewMode::MyIssues => "Assigned",
            ViewMode::Mentions => "Mentions",
            ViewMode::Watching => "Watched",
            ViewMode::Updated => "Updated",
            ViewMode::Sprint => "Sprint",
            ViewMode::ClosedSearch => "Closed",
        }
    }

    pub fn cache_key(&self) -> &'static str {
        match self {
            ViewMode::MyIssues => "assigned",
            ViewMode::Mentions => "mentions",
            ViewMode::Watching => "watched",
            ViewMode::Updated => "updated",
            ViewMode::Sprint => "sprint",
            ViewMode::ClosedSearch => "closed",
        }
    }

    /// Tab order: Assigned → Mentions → Watched → Updated → Sprint → Closed.
    pub fn all() -> [ViewMode; 6] {
        [
            ViewMode::MyIssues,
            ViewMode::Mentions,
            ViewMode::Watching,
            ViewMode::Updated,
            ViewMode::Sprint,
            ViewMode::ClosedSearch,
        ]
    }

    /// Views refreshed in the background (excludes on-demand closed search).
    pub fn background() -> [ViewMode; 5] {
        [
            ViewMode::MyIssues,
            ViewMode::Mentions,
            ViewMode::Watching,
            ViewMode::Updated,
            ViewMode::Sprint,
        ]
    }

    pub fn prefetches_on_startup(self) -> bool {
        self != ViewMode::ClosedSearch
    }
}

/// Escape a user string for Jira `~` text clauses.
pub fn escape_jql_text(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Build closed-tab JQL from base clause and search words.
pub fn build_closed_search_jql(base: &str, query: &str) -> String {
    let q = query.trim();
    if q.is_empty() {
        return format!("{base} ORDER BY updated DESC");
    }
    let escaped = escape_jql_text(q);
    format!("{base} AND text ~ \"{escaped}\" ORDER BY updated DESC")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tab_order_matches_requested_sequence() {
        let tabs = ViewMode::all();
        assert_eq!(tabs[0], ViewMode::MyIssues);
        assert_eq!(tabs[1], ViewMode::Mentions);
        assert_eq!(tabs[2], ViewMode::Watching);
        assert_eq!(tabs[3], ViewMode::Updated);
        assert_eq!(tabs[4], ViewMode::Sprint);
        assert_eq!(tabs[5], ViewMode::ClosedSearch);
    }

    #[test]
    fn closed_jql_includes_text_and_was_assignee() {
        let jql = build_closed_search_jql(ViewMode::closed_search_base(true), "payment bug");
        assert!(jql.contains("assignee was currentUser()"));
        assert!(jql.contains("text ~ \"payment bug\""));
    }

    #[test]
    fn escape_jql_quotes() {
        assert_eq!(escape_jql_text(r#"say "hi""#), r#"say \"hi\""#);
    }
}
