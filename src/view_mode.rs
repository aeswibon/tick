#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ViewMode {
    MyIssues,
    Updated,
    Mentions,
    Watching,
    Sprint,
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
            ViewMode::Sprint => {
                "sprint in openSprints() AND assignee = currentUser() ORDER BY updated DESC"
            }
        }
    }

    pub fn next(self) -> Self {
        match self {
            ViewMode::MyIssues => ViewMode::Updated,
            ViewMode::Updated => ViewMode::Mentions,
            ViewMode::Mentions => ViewMode::Watching,
            ViewMode::Watching => ViewMode::Sprint,
            ViewMode::Sprint => ViewMode::MyIssues,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            ViewMode::MyIssues => ViewMode::Sprint,
            ViewMode::Updated => ViewMode::MyIssues,
            ViewMode::Mentions => ViewMode::Updated,
            ViewMode::Watching => ViewMode::Mentions,
            ViewMode::Sprint => ViewMode::Watching,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            ViewMode::MyIssues => "Assigned",
            ViewMode::Updated => "Updated",
            ViewMode::Mentions => "Mentions",
            ViewMode::Watching => "Watched",
            ViewMode::Sprint => "Sprint",
        }
    }

    pub fn cache_key(&self) -> &'static str {
        match self {
            ViewMode::MyIssues => "assigned",
            ViewMode::Updated => "updated",
            ViewMode::Mentions => "mentions",
            ViewMode::Watching => "watched",
            ViewMode::Sprint => "sprint",
        }
    }

    pub fn all() -> [ViewMode; 5] {
        [
            ViewMode::MyIssues,
            ViewMode::Updated,
            ViewMode::Mentions,
            ViewMode::Watching,
            ViewMode::Sprint,
        ]
    }
}
