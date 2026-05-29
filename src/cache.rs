//! Per-view JSON cache under `~/.config/tick/cache/`.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::api::types::{CachedView, Ticket};
use crate::view_mode::ViewMode;

pub struct ViewCache {
    pub(crate) dir: PathBuf,
}

impl ViewCache {
    pub fn open() -> Self {
        let dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("tick")
            .join("cache");
        let _ = fs::create_dir_all(&dir);
        Self { dir }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn load_all(&self) -> HashMap<ViewMode, Vec<Ticket>> {
        let mut map = HashMap::new();
        for mode in ViewMode::all() {
            if let Some(tickets) = self.load_view(mode) {
                map.insert(mode, tickets);
            }
        }
        map
    }

    pub fn load_view(&self, mode: ViewMode) -> Option<Vec<Ticket>> {
        let content = fs::read_to_string(self.path_for(mode)).ok()?;
        let cached: CachedView = serde_json::from_str(&content).ok()?;
        Some(cached.tickets)
    }

    pub fn save_view(&self, mode: ViewMode, tickets: &[Ticket]) {
        let cached = CachedView {
            fetched_at: chrono::Utc::now().to_rfc3339(),
            tickets: tickets.to_vec(),
        };
        if let Ok(content) = serde_json::to_string(&cached) {
            let _ = fs::write(self.path_for(mode), content);
        }
    }

    fn path_for(&self, mode: ViewMode) -> PathBuf {
        self.dir.join(format!("{}.json", mode.cache_key()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::types::Ticket;

    fn sample_ticket(key: &str) -> Ticket {
        Ticket {
            key: key.into(),
            site: "s".into(),
            issue_type: "Task".into(),
            status: "Open".into(),
            status_color: "blue".into(),
            priority: "Medium".into(),
            ageing_days: 1,
            due_date: None,
            assignee: String::new(),
            reporter: String::new(),
            summary: "x".into(),
            link: String::new(),
            description: None,
            description_adf: None,
            latest_comment: None,
            all_comments: vec![],
            parent_key: None,
            parent_summary: None,
            labels: vec![],
            sprint_name: None,
        }
    }

    #[test]
    fn roundtrip_save_and_load() {
        let dir = std::env::temp_dir().join(format!("tick-cache-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let cache = ViewCache { dir: dir.clone() };
        let tickets = vec![sample_ticket("A-1"), sample_ticket("A-2")];
        cache.save_view(ViewMode::MyIssues, &tickets);
        let loaded = cache.load_view(ViewMode::MyIssues).unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].key, "A-1");
        let _ = fs::remove_dir_all(dir);
    }
}
