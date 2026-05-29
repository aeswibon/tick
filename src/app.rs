use crate::api::{self, types::{CachedView, Ticket}};
use crate::columns::Column;
use crate::config::Config;
use crate::fetch_status::FetchStatus;
use crate::theme::Theme;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;

type BgResult = Vec<(ViewMode, Vec<Ticket>, Vec<String>)>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortMode {
    Default,
    Age,
    Priority,
    Status,
    Key,
}

impl SortMode {
    pub fn next(self) -> Self {
        match self {
            Self::Default => Self::Age,
            Self::Age => Self::Priority,
            Self::Priority => Self::Status,
            Self::Status => Self::Key,
            Self::Key => Self::Default,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Age => "age",
            Self::Priority => "priority",
            Self::Status => "status",
            Self::Key => "key",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DetailTab {
    Details,
    Description,
    Comments,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ViewMode {
    MyIssues,
    Updated,
    Mentions,
    Watching,
}

impl ViewMode {
    pub fn jql(&self) -> &'static str {
        match self {
            ViewMode::MyIssues => "assignee = currentUser() AND statusCategory != Done ORDER BY updated DESC",
            ViewMode::Updated => "assignee = currentUser() AND statusCategory != Done AND updated >= -7d ORDER BY updated DESC",
            ViewMode::Mentions => "comment ~ currentUser() AND statusCategory != Done ORDER BY updated DESC",
            ViewMode::Watching => "watcher = currentUser() AND statusCategory != Done ORDER BY updated DESC",
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
        [ViewMode::MyIssues, ViewMode::Updated, ViewMode::Mentions, ViewMode::Watching]
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputMode {
    None,
    Comment,
    Worklog,
}

pub struct App {
    pub tickets: Arc<RwLock<Vec<Ticket>>>,
    pub last_refresh: Instant,
    pub loading: bool,
    pub status: FetchStatus,
    pub columns: Vec<Column>,
    pub selected: usize,
    pub config: Config,
    pub theme: Theme,
    pub detail_open: bool,
    pub detail_tab: DetailTab,
    pub show_help: bool,
    pub showing_transitions: bool,
    pub transition_options: Vec<(String, String)>,
    pub filter: String,
    pub filtering: bool,
    pub sort_mode: SortMode,
    pub input_mode: InputMode,
    pub input_buffer: String,
    pub debug: bool,
    pub current_page: usize,
    pub page_size: usize,
    pub active_view: ViewMode,
    pub view_cache: HashMap<ViewMode, Vec<Ticket>>,
    pub cache_dir: PathBuf,
    pub pending_bg_update: Arc<Mutex<Option<BgResult>>>,
}

impl App {
    pub fn new(config: Config, theme: Theme, debug: bool) -> Self {
        let cache_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("tick").join("cache");
        let _ = std::fs::create_dir_all(&cache_dir);

        let columns = Column::resolve(config.columns.as_deref());

        let mut app = Self {
            tickets: Arc::new(RwLock::new(Vec::new())),
            last_refresh: Instant::now(),
            loading: true,
            status: FetchStatus::default(),
            columns,
            selected: 0,
            config,
            theme,
            detail_open: false,
            detail_tab: DetailTab::Details,
            show_help: false,
            showing_transitions: false,
            transition_options: Vec::new(),
            filter: String::new(),
            filtering: false,
            sort_mode: SortMode::Default,
            input_mode: InputMode::None,
            input_buffer: String::new(),
            debug,
            current_page: 0,
            page_size: 10,
            active_view: ViewMode::MyIssues,
            view_cache: HashMap::new(),
            cache_dir,
            pending_bg_update: Arc::new(Mutex::new(None)),
        };
        app.load_cache();
        app
    }

    fn load_cache(&mut self) {
        for mode in ViewMode::all() {
            let path = self.cache_dir.join(format!("{}.json", mode.cache_key()));
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(cached) = serde_json::from_str::<CachedView>(&content) {
                    self.view_cache.insert(mode, cached.tickets);
                }
            }
        }
        if let Some(cached) = self.view_cache.get(&self.active_view).cloned() {
            if let Ok(mut data) = self.tickets.write() {
                *data = cached;
            }
            self.loading = false;
        }
    }

    pub fn save_cache(&self, mode: ViewMode) {
        if let Some(tickets) = self.view_cache.get(&mode) {
            let cached = CachedView {
                fetched_at: chrono::Utc::now().to_rfc3339(),
                tickets: tickets.clone(),
            };
            if let Ok(content) = serde_json::to_string(&cached) {
                let path = self.cache_dir.join(format!("{}.json", mode.cache_key()));
                let _ = std::fs::write(&path, &content);
            }
        }
    }

    pub fn visible_indices(&self) -> Vec<usize> {
        let all = self.filtered_indices();
        let total_pages = self.total_pages();
        let page = self.current_page.min(total_pages.saturating_sub(1));
        let start = page * self.page_size;
        let end = start + self.page_size;
        if start >= all.len() {
            Vec::new()
        } else {
            all[start..end.min(all.len())].to_vec()
        }
    }

    pub fn total_pages(&self) -> usize {
        let total = self.filtered_count();
        if total == 0 { 1 } else { (total + self.page_size - 1) / self.page_size }
    }

    pub fn next_page(&mut self) {
        let total = self.total_pages();
        if self.current_page + 1 < total {
            self.current_page += 1;
            self.selected = 0;
        }
    }

    pub fn prev_page(&mut self) {
        if self.current_page > 0 {
            self.current_page -= 1;
            self.selected = 0;
        }
    }

    pub fn go_to_last(&mut self) {
        let total = self.total_pages();
        self.current_page = total.saturating_sub(1);
        let last_count = self.visible_indices().len();
        self.selected = last_count.saturating_sub(1);
    }

    pub async fn refresh(&mut self) {
        self.loading = true;
        let jql = self.active_view.jql();
        let (tickets, errors) = api::fetch_all(&self.config, self.debug, jql).await;
        self.status.set_site_warnings(errors);
        if errors.is_empty() {
            self.status.clear_action_error();
        }
        self.view_cache.insert(self.active_view, tickets.clone());
        self.save_cache(self.active_view);
        {
            let mut data = self.tickets.write().unwrap();
            *data = tickets;
        }
        self.loading = false;
        self.last_refresh = Instant::now();
        self.current_page = 0;
        self.selected = 0;
    }

    pub async fn refresh_all(&mut self) {
        let views = ViewMode::all();
        self.do_refresh_all(views).await;
    }

    async fn do_refresh_all(&mut self, views: [ViewMode; 4]) {
        let (r0, r1, r2, r3) = tokio::join!(
            api::fetch_all(&self.config, self.debug, views[0].jql()),
            api::fetch_all(&self.config, self.debug, views[1].jql()),
            api::fetch_all(&self.config, self.debug, views[2].jql()),
            api::fetch_all(&self.config, self.debug, views[3].jql()),
        );
        let results = [r0, r1, r2, r3];
        let mut all_errors = Vec::new();
        for (i, (tickets, errs)) in results.into_iter().enumerate() {
            let mode = views[i];
            self.view_cache.insert(mode, tickets.clone());
            self.save_cache(mode);
            if mode == self.active_view {
                if let Ok(mut data) = self.tickets.write() {
                    *data = tickets;
                }
            }
            all_errors.extend(errs);
        }
        self.status.set_site_warnings(all_errors);
        self.loading = false;
        self.last_refresh = Instant::now();
        self.current_page = 0;
        self.selected = 0;
    }

    pub fn spawn_background_refresh(&self) {
        let config = self.config.clone();
        let debug = self.debug;
        let pending = self.pending_bg_update.clone();
        let views = ViewMode::all();
        tokio::spawn(async move {
            let (r0, r1, r2, r3) = tokio::join!(
                api::fetch_all(&config, debug, views[0].jql()),
                api::fetch_all(&config, debug, views[1].jql()),
                api::fetch_all(&config, debug, views[2].jql()),
                api::fetch_all(&config, debug, views[3].jql()),
            );
            let results = vec![
                (views[0], r0.0, r0.1),
                (views[1], r1.0, r1.1),
                (views[2], r2.0, r2.1),
                (views[3], r3.0, r3.1),
            ];
            *pending.lock().unwrap() = Some(results);
        });
    }

    pub fn apply_pending_updates(&mut self) -> bool {
        let updates = self.pending_bg_update.lock().unwrap().take();
        if let Some(results) = updates {
            let mut all_errors = Vec::new();
            for (mode, tickets, errs) in results {
                self.view_cache.insert(mode, tickets.clone());
                self.save_cache(mode);
                if mode == self.active_view {
                    if let Ok(mut data) = self.tickets.write() {
                        *data = tickets;
                    }
                }
                all_errors.extend(errs);
            }
            self.status.set_site_warnings(all_errors);
            self.loading = false;
            self.last_refresh = Instant::now();
            self.current_page = 0;
            self.selected = 0;
            true
        } else {
            false
        }
    }

    pub async fn switch_to(&mut self, mode: ViewMode) {
        self.active_view = mode;
        if let Some(cached) = self.view_cache.get(&mode).cloned() {
            if let Ok(mut data) = self.tickets.write() {
                *data = cached;
            }
            self.loading = false;
            self.current_page = 0;
            self.selected = 0;
        } else {
            let path = self.cache_dir.join(format!("{}.json", mode.cache_key()));
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(cached) = serde_json::from_str::<CachedView>(&content) {
                    if let Ok(mut data) = self.tickets.write() {
                        *data = cached.tickets.clone();
                    }
                    self.view_cache.insert(mode, cached.tickets);
                    self.loading = false;
                    self.current_page = 0;
                    self.selected = 0;
                    self.detail_open = false;
                    return;
                }
            }
            self.refresh().await;
        }
        self.detail_open = false;
    }

    pub fn sites_str(&self) -> String {
        self.config.sites.iter().map(|s| {
            let count = self.site_count(&s.name);
            format!("{}: {}", s.name, count)
        }).collect::<Vec<_>>().join(", ")
    }

    #[allow(dead_code)]
    pub fn total_tickets(&self) -> usize {
        self.tickets.read().unwrap().len()
    }

    pub fn site_count(&self, site: &str) -> usize {
        self.tickets.read().unwrap().iter()
            .filter(|t| t.site == site)
            .count()
    }

    pub fn filtered_indices(&self) -> Vec<usize> {
        let tickets = self.tickets.read().unwrap();
        let mut indices: Vec<usize> = if self.filter.is_empty() {
            (0..tickets.len()).collect()
        } else {
            let q = self.filter.to_lowercase();
            tickets.iter().enumerate()
                .filter(|(_, t)| {
                    t.key.to_lowercase().contains(&q)
                        || t.summary.to_lowercase().contains(&q)
                        || t.status.to_lowercase().contains(&q)
                        || t.assignee.to_lowercase().contains(&q)
                        || t.issue_type.to_lowercase().contains(&q)
                        || t.site.to_lowercase().contains(&q)
                        || t.priority.to_lowercase().contains(&q)
                })
                .map(|(i, _)| i)
                .collect()
        };

        match self.sort_mode {
            SortMode::Default => {}
            SortMode::Age => indices.sort_by(|&a, &b| tickets[a].ageing_days.cmp(&tickets[b].ageing_days)),
            SortMode::Priority => {
                let prio = |p: &str| -> u8 {
                    match p {
                        "Highest" | "Blocker" | "P1" => 1,
                        "High" | "Critical" | "P2" => 2,
                        "Medium" | "Major" | "P3" => 3,
                        "Low" | "Minor" | "P4" => 4,
                        "Lowest" | "Trivial" | "P5" => 5,
                        _ => 9,
                    }
                };
                indices.sort_by(|&a, &b| prio(&tickets[a].priority).cmp(&prio(&tickets[b].priority)));
            }
            SortMode::Status => indices.sort_by(|&a, &b| tickets[a].status_color.cmp(&tickets[b].status_color)),
            SortMode::Key => indices.sort_by(|&a, &b| tickets[a].key.cmp(&tickets[b].key)),
        }

        indices
    }

    pub fn filtered_count(&self) -> usize {
        self.filtered_indices().len()
    }
}
