#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ViewMode {
    MyIssues,
    Updated,
    Mentions,
    Watching,
}

impl ViewMode {
    pub fn default_jql(&self) -> &'static str {
        match self {
            ViewMode::MyIssues => {
                "assignee = currentUser() AND statusCategory != Done ORDER BY updated DESC"
            }
            ViewMode::Updated => {
                "assignee = currentUser() AND statusCategory != Done AND updated >= -7d ORDER BY updated DESC"
            }
            ViewMode::Mentions => {
                "comment ~ currentUser() AND statusCategory != Done ORDER BY updated DESC"
            }
            ViewMode::Watching => {
                "watcher = currentUser() AND statusCategory != Done ORDER BY updated DESC"
            }
        }
    }

    pub fn next(self) -> Self {
        match self {
            ViewMode::MyIssues => ViewMode::Updated,
            ViewMode::Updated => ViewMode::Mentions,
            ViewMode::Mentions => ViewMode::Watching,
            ViewMode::Watching => ViewMode::MyIssues,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            ViewMode::MyIssues => ViewMode::Watching,
            ViewMode::Updated => ViewMode::MyIssues,
            ViewMode::Mentions => ViewMode::Updated,
            ViewMode::Watching => ViewMode::Mentions,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            ViewMode::MyIssues => "Assigned",
            ViewMode::Updated => "Updated",
            ViewMode::Mentions => "Mentions",
            ViewMode::Watching => "Watched",
        }
    }

    pub fn cache_key(&self) -> &'static str {
        match self {
            ViewMode::MyIssues => "assigned",
            ViewMode::Updated => "updated",
            ViewMode::Mentions => "mentions",
            ViewMode::Watching => "watched",
        }
    }

    pub fn all() -> [ViewMode; 4] {
        [
            ViewMode::MyIssues,
            ViewMode::Updated,
            ViewMode::Mentions,
            ViewMode::Watching,
        ]
    }
}
