//! Per-view JSON cache under `~/.config/tick/cache/`.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::api::types::{CachedView, Ticket};
use crate::view_mode::ViewMode;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClosedPrefs {
    pub query: String,
    pub ever_assigned: bool,
}

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
        self.load_cached_view(mode).map(|c| c.tickets)
    }

    pub fn load_cached_view(&self, mode: ViewMode) -> Option<CachedView> {
        let content = fs::read_to_string(self.path_for(mode)).ok()?;
        serde_json::from_str(&content).ok()
    }

    pub fn fetched_at_for(&self, mode: ViewMode) -> Option<DateTime<Utc>> {
        let cached = self.load_cached_view(mode)?;
        DateTime::parse_from_rfc3339(&cached.fetched_at)
            .ok()
            .map(|dt| dt.with_timezone(&Utc))
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

    fn custom_path(&self, slug: &str) -> PathBuf {
        self.dir.join(format!("custom-{slug}.json"))
    }

    pub fn load_custom_view(&self, slug: &str) -> Option<Vec<Ticket>> {
        let content = fs::read_to_string(self.custom_path(slug)).ok()?;
        serde_json::from_str::<CachedView>(&content)
            .ok()
            .map(|c| c.tickets)
    }

    pub fn save_custom_view(&self, slug: &str, tickets: &[Ticket]) {
        let cached = CachedView {
            fetched_at: chrono::Utc::now().to_rfc3339(),
            tickets: tickets.to_vec(),
        };
        if let Ok(content) = serde_json::to_string(&cached) {
            let _ = fs::write(self.custom_path(slug), content);
        }
    }

    pub fn closed_prefs_path(&self) -> PathBuf {
        self.dir.join("closed_prefs.json")
    }

    pub fn load_closed_prefs(&self) -> ClosedPrefs {
        let path = self.closed_prefs_path();
        let Ok(content) = fs::read_to_string(path) else {
            return ClosedPrefs::default();
        };
        serde_json::from_str(&content).unwrap_or_default()
    }

    pub fn save_closed_prefs(&self, prefs: &ClosedPrefs) {
        if let Ok(content) = serde_json::to_string(prefs) {
            let _ = fs::write(self.closed_prefs_path(), content);
        }
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
            project_key: String::new(),
            custom_fields: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn fetched_at_roundtrip() {
        let dir = std::env::temp_dir().join(format!("tick-cache-age-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let cache = ViewCache { dir: dir.clone() };
        cache.save_view(ViewMode::MyIssues, &[sample_ticket("A-1")]);
        assert!(cache.fetched_at_for(ViewMode::MyIssues).is_some());
        let _ = fs::remove_dir_all(dir);
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
