use crate::api::types::Ticket;
use crate::api::{self, JiraClient};
use crate::cache::ViewCache;
use crate::columns::Column;
use crate::config::Config;
use crate::fetch_status::FetchStatus;
use crate::theme::Theme;
use crate::ticket_lock::{read_tickets, write_tickets};
use crate::view_mode::ViewMode;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;

type BgResult = Vec<(ViewMode, Vec<Ticket>, Vec<String>)>;

#[derive(Debug, Clone)]
pub struct TicketRef {
    pub site: String,
    pub key: String,
    pub link: String,
}

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

impl DetailTab {
    pub fn next(self) -> Self {
        match self {
            Self::Details => Self::Description,
            Self::Description => Self::Comments,
            Self::Comments => Self::Details,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Details => Self::Comments,
            Self::Description => Self::Details,
            Self::Comments => Self::Description,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputMode {
    None,
    Comment,
    Worklog,
    EditSummary,
}

struct FilterCache {
    filter: String,
    sort_mode: SortMode,
    ticket_count: usize,
    indices: Vec<usize>,
}

pub struct App {
    pub tickets: Arc<RwLock<Vec<Ticket>>>,
    pub jira: Arc<JiraClient>,
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
    pub transition_selected: usize,
    pub transition_options: Vec<(String, String)>,
    pub showing_priorities: bool,
    pub priority_selected: usize,
    pub priority_options: Vec<(String, String)>,
    pub show_site_errors: bool,
    pub site_error_scroll: usize,
    pub live_data: bool,
    pub filter: String,
    pub filtering: bool,
    pub sort_mode: SortMode,
    pub input_mode: InputMode,
    pub input_buffer: String,
    #[allow(dead_code)]
    pub debug: bool,
    pub current_page: usize,
    pub page_size: usize,
    pub active_view: ViewMode,
    pub view_cache: HashMap<ViewMode, Vec<Ticket>>,
    cache: ViewCache,
    pub pending_bg_update: Arc<Mutex<Option<BgResult>>>,
    filter_cache: RefCell<Option<FilterCache>>,
}

impl App {
    pub fn new(config: Config, theme: Theme, debug: bool) -> Self {
        let cache = ViewCache::open();
        let jira = Arc::new(JiraClient::new(&config.email, &config.token, debug));
        let columns = Column::resolve(config.columns.as_deref());
        let page_size = config.page_size as usize;

        let mut app = Self {
            tickets: Arc::new(RwLock::new(Vec::new())),
            jira,
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
            transition_selected: 0,
            transition_options: Vec::new(),
            showing_priorities: false,
            priority_selected: 0,
            priority_options: Vec::new(),
            show_site_errors: false,
            site_error_scroll: 0,
            live_data: false,
            filter: String::new(),
            filtering: false,
            sort_mode: SortMode::Default,
            input_mode: InputMode::None,
            input_buffer: String::new(),
            debug,
            current_page: 0,
            page_size,
            active_view: ViewMode::MyIssues,
            view_cache: HashMap::new(),
            cache,
            pending_bg_update: Arc::new(Mutex::new(None)),
            filter_cache: RefCell::new(None),
        };
        app.load_cache();
        app
    }

    pub fn invalidate_filter_cache(&self) {
        *self.filter_cache.borrow_mut() = None;
    }

    pub fn site_base_url(&self, site_name: &str) -> Option<String> {
        self.config
            .sites
            .iter()
            .find(|s| s.name == site_name)
            .map(|s| s.base_url.clone())
    }

    pub fn selected_ticket(&self) -> Option<TicketRef> {
        self.selected_ticket_entry().map(|t| TicketRef {
            site: t.site,
            key: t.key,
            link: t.link,
        })
    }

    pub fn selected_ticket_entry(&self) -> Option<Ticket> {
        let tickets = read_tickets(&self.tickets);
        let indices = self.visible_indices();
        let ticket_idx = *indices.get(self.selected)?;
        tickets.get(ticket_idx).cloned()
    }

    fn load_cache(&mut self) {
        self.view_cache = self.cache.load_all();
        if let Some(cached) = self.view_cache.get(&self.active_view).cloned() {
            write_tickets(&self.tickets).clone_from(&cached);
            self.loading = false;
            self.live_data = false;
            self.invalidate_filter_cache();
        }
    }

    pub fn save_cache(&self, mode: ViewMode) {
        if let Some(tickets) = self.view_cache.get(&mode) {
            self.cache.save_view(mode, tickets);
        }
    }

    pub fn visible_indices(&self) -> Vec<usize> {
        let all = self.filtered_indices();
        let total_pages = self.total_pages_from_len(all.len());
        let page = self.current_page.min(total_pages.saturating_sub(1));
        let start = page * self.page_size;
        let end = start + self.page_size;
        if start >= all.len() {
            Vec::new()
        } else {
            all[start..end.min(all.len())].to_vec()
        }
    }

    fn total_pages_from_len(&self, total: usize) -> usize {
        if total == 0 {
            1
        } else {
            total.div_ceil(self.page_size)
        }
    }

    pub fn total_pages(&self) -> usize {
        self.total_pages_from_len(self.filtered_count())
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

    fn apply_fetch_result(
        &mut self,
        tickets: Vec<Ticket>,
        errors: Vec<String>,
        reset_cursor: bool,
    ) {
        let no_errors = errors.is_empty();
        let had_tickets = !read_tickets(&self.tickets).is_empty();

        if tickets.is_empty() && !no_errors && had_tickets {
            self.status.set_site_warnings(errors);
            self.live_data = false;
            return;
        }

        self.status.set_site_warnings(errors);
        if no_errors {
            self.status.clear_action_error();
        }
        write_tickets(&self.tickets).clone_from(&tickets);
        self.invalidate_filter_cache();
        self.live_data = !tickets.is_empty() || no_errors;
        if reset_cursor {
            self.current_page = 0;
            self.selected = 0;
        }
    }

    pub async fn refresh(&mut self) {
        self.loading = true;
        let jql = self.config.jql_for(self.active_view);
        let (tickets, errors) = api::fetch_all(&self.jira, &self.config, jql).await;
        self.view_cache.insert(self.active_view, tickets.clone());
        self.save_cache(self.active_view);
        self.apply_fetch_result(tickets, errors, true);
        self.loading = false;
        self.last_refresh = Instant::now();
    }

    pub async fn refresh_all(&mut self) {
        self.do_refresh_all(ViewMode::all()).await;
    }

    async fn do_refresh_all(&mut self, views: [ViewMode; 4]) {
        self.loading = true;
        let (r0, r1, r2, r3) = tokio::join!(
            api::fetch_all(&self.jira, &self.config, self.config.jql_for(views[0])),
            api::fetch_all(&self.jira, &self.config, self.config.jql_for(views[1])),
            api::fetch_all(&self.jira, &self.config, self.config.jql_for(views[2])),
            api::fetch_all(&self.jira, &self.config, self.config.jql_for(views[3])),
        );
        let results = [r0, r1, r2, r3];
        let mut all_errors = Vec::new();
        let mut active_tickets = None;
        for (i, (tickets, errs)) in results.into_iter().enumerate() {
            let mode = views[i];
            self.view_cache.insert(mode, tickets.clone());
            self.save_cache(mode);
            if mode == self.active_view {
                active_tickets = Some(tickets);
            }
            all_errors.extend(errs);
        }
        if let Some(tickets) = active_tickets {
            self.apply_fetch_result(tickets, all_errors, true);
        } else {
            self.status.set_site_warnings(all_errors);
        }
        self.loading = false;
        self.last_refresh = Instant::now();
    }

    pub fn spawn_background_refresh(&self) {
        let jira = self.jira.clone();
        let config = self.config.clone();
        let pending = self.pending_bg_update.clone();
        let views = ViewMode::all();
        tokio::spawn(async move {
            let (r0, r1, r2, r3) = tokio::join!(
                api::fetch_all(&jira, &config, config.jql_for(views[0])),
                api::fetch_all(&jira, &config, config.jql_for(views[1])),
                api::fetch_all(&jira, &config, config.jql_for(views[2])),
                api::fetch_all(&jira, &config, config.jql_for(views[3])),
            );
            let results = vec![
                (views[0], r0.0, r0.1),
                (views[1], r1.0, r1.1),
                (views[2], r2.0, r2.1),
                (views[3], r3.0, r3.1),
            ];
            if let Ok(mut slot) = pending.lock() {
                *slot = Some(results);
            }
        });
    }

    pub fn apply_pending_updates(&mut self) -> bool {
        let updates = self
            .pending_bg_update
            .lock()
            .ok()
            .and_then(|mut g| g.take());
        if let Some(results) = updates {
            let previous_keys = ticket_keys(&read_tickets(&self.tickets));
            let mut all_errors = Vec::new();
            let mut active_tickets = None;
            for (mode, tickets, errs) in results {
                self.view_cache.insert(mode, tickets.clone());
                self.save_cache(mode);
                if mode == self.active_view {
                    active_tickets = Some(tickets);
                }
                all_errors.extend(errs);
            }
            if let Some(tickets) = active_tickets {
                let preserve_ui = same_ticket_keys(&previous_keys, &tickets);
                self.apply_fetch_result(tickets, all_errors, !preserve_ui);
                if preserve_ui {
                    self.clamp_selection();
                }
            } else {
                self.status.set_site_warnings(all_errors);
            }
            self.loading = false;
            self.last_refresh = Instant::now();
            true
        } else {
            false
        }
    }

    pub async fn switch_to(&mut self, mode: ViewMode) {
        self.active_view = mode;
        if let Some(cached) = self.view_cache.get(&mode).cloned() {
            write_tickets(&self.tickets).clone_from(&cached);
            self.loading = false;
            self.live_data = false;
            self.invalidate_filter_cache();
            self.current_page = 0;
            self.selected = 0;
        } else if let Some(tickets) = self.cache.load_view(mode) {
            write_tickets(&self.tickets).clone_from(&tickets);
            self.view_cache.insert(mode, tickets.clone());
            self.loading = false;
            self.live_data = false;
            self.invalidate_filter_cache();
            self.current_page = 0;
            self.selected = 0;
            self.detail_open = false;
            return;
        } else {
            self.refresh().await;
        }
        self.detail_open = false;
    }

    pub fn sites_str(&self) -> String {
        self.config
            .sites
            .iter()
            .map(|s| {
                let count = self.site_count(&s.name);
                format!("{}: {}", s.name, count)
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    pub fn site_count(&self, site: &str) -> usize {
        read_tickets(&self.tickets)
            .iter()
            .filter(|t| t.site == site)
            .count()
    }

    pub fn filtered_indices(&self) -> Vec<usize> {
        let tickets = read_tickets(&self.tickets);
        let ticket_count = tickets.len();

        if let Some(cache) = self.filter_cache.borrow().as_ref() {
            if cache.filter == self.filter
                && cache.sort_mode == self.sort_mode
                && cache.ticket_count == ticket_count
            {
                return cache.indices.clone();
            }
        }

        let indices = compute_filtered_indices(&tickets, &self.filter, self.sort_mode);
        drop(tickets);

        *self.filter_cache.borrow_mut() = Some(FilterCache {
            filter: self.filter.clone(),
            sort_mode: self.sort_mode,
            ticket_count,
            indices: indices.clone(),
        });

        indices
    }

    fn clamp_selection(&mut self) {
        let visible = self.visible_indices().len();
        if visible == 0 {
            self.selected = 0;
            self.current_page = 0;
            return;
        }
        let total_pages = self.total_pages();
        if self.current_page >= total_pages {
            self.current_page = total_pages.saturating_sub(1);
        }
        if self.selected >= visible {
            self.selected = visible.saturating_sub(1);
        }
    }

    pub fn filtered_count(&self) -> usize {
        let ticket_count = read_tickets(&self.tickets).len();
        if let Some(cache) = self.filter_cache.borrow().as_ref() {
            if cache.filter == self.filter
                && cache.sort_mode == self.sort_mode
                && cache.ticket_count == ticket_count
            {
                return cache.indices.len();
            }
        }
        self.filtered_indices().len()
    }
}

fn ticket_keys(tickets: &[Ticket]) -> Vec<String> {
    tickets.iter().map(|t| t.key.clone()).collect()
}

fn same_ticket_keys(previous: &[String], tickets: &[Ticket]) -> bool {
    if previous.len() != tickets.len() {
        return false;
    }
    let mut a = previous.to_vec();
    let mut b: Vec<_> = tickets.iter().map(|t| t.key.clone()).collect();
    a.sort();
    b.sort();
    a == b
}

fn compute_filtered_indices(tickets: &[Ticket], filter: &str, sort_mode: SortMode) -> Vec<usize> {
    let mut indices: Vec<usize> = if filter.is_empty() {
        (0..tickets.len()).collect()
    } else {
        let q = filter.to_lowercase();
        tickets
            .iter()
            .enumerate()
            .filter(|(_, t)| {
                t.key.to_lowercase().contains(&q)
                    || t.summary.to_lowercase().contains(&q)
                    || t.status.to_lowercase().contains(&q)
                    || t.assignee.to_lowercase().contains(&q)
                    || t.issue_type.to_lowercase().contains(&q)
                    || t.site.to_lowercase().contains(&q)
                    || t.priority.to_lowercase().contains(&q)
                    || t.parent_key
                        .as_ref()
                        .is_some_and(|p| p.to_lowercase().contains(&q))
                    || t.labels.iter().any(|l| l.to_lowercase().contains(&q))
                    || t.sprint_name
                        .as_ref()
                        .is_some_and(|s| s.to_lowercase().contains(&q))
            })
            .map(|(i, _)| i)
            .collect()
    };

    match sort_mode {
        SortMode::Default => {}
        SortMode::Age => {
            indices.sort_by(|&a, &b| tickets[a].ageing_days.cmp(&tickets[b].ageing_days));
        }
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
        SortMode::Status => {
            indices.sort_by(|&a, &b| tickets[a].status.cmp(&tickets[b].status));
        }
        SortMode::Key => indices.sort_by(|&a, &b| tickets[a].key.cmp(&tickets[b].key)),
    }

    indices
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::types::Ticket;
    fn sample_ticket(key: &str, summary: &str, status: &str) -> Ticket {
        Ticket {
            key: key.into(),
            site: "acme".into(),
            issue_type: "Task".into(),
            status: status.into(),
            status_color: "yellow".into(),
            priority: "Medium".into(),
            ageing_days: 1,
            due_date: None,
            assignee: "Alice".into(),
            reporter: "Bob".into(),
            summary: summary.into(),
            link: format!("https://acme.atlassian.net/browse/{key}"),
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
    fn filter_matches_labels() {
        let mut t1 = sample_ticket("A-1", "one", "Open");
        t1.labels = vec!["backend".into()];
        let t2 = sample_ticket("A-2", "two", "Open");
        let tickets = vec![t1, t2];
        let idx = compute_filtered_indices(&tickets, "backend", SortMode::Default);
        assert_eq!(idx, vec![0]);
    }

    #[test]
    fn filter_matches_summary() {
        let tickets = vec![
            sample_ticket("A-1", "login bug", "Open"),
            sample_ticket("A-2", "billing", "Open"),
        ];
        let idx = compute_filtered_indices(&tickets, "login", SortMode::Default);
        assert_eq!(idx, vec![0]);
    }

    #[test]
    fn new_uses_config_page_size() {
        let config = Config {
            email: "a@b.com".into(),
            token: "t".into(),
            sites: vec![crate::config::Site {
                name: "acme".into(),
                base_url: "https://acme.atlassian.net".into(),
                sprint_field: None,
            }],
            columns: None,
            max_results: 50,
            page_size: 7,
            theme: "default".into(),
            views: Default::default(),
            view_jql: Config::build_view_jql(&Default::default()),
        };
        let app = App::new(config, Theme::default(), false);
        assert_eq!(app.page_size, 7);
    }

    #[test]
    fn pagination_slices_filtered_list() {
        let config = Config {
            email: "a@b.com".into(),
            token: "t".into(),
            sites: vec![crate::config::Site {
                name: "acme".into(),
                base_url: "https://acme.atlassian.net".into(),
                sprint_field: None,
            }],
            columns: None,
            max_results: 50,
            page_size: 10,
            theme: "default".into(),
            views: Default::default(),
            view_jql: Config::build_view_jql(&Default::default()),
        };
        let theme = Theme::default();
        let mut app = App::new(config, theme, false);
        app.page_size = 2;
        *write_tickets(&app.tickets) = (0..5)
            .map(|i| sample_ticket(&format!("T-{i}"), "x", "Open"))
            .collect();
        app.invalidate_filter_cache();
        app.current_page = 1;
        assert_eq!(app.visible_indices(), vec![2, 3]);
        assert_eq!(app.total_pages(), 3);
    }

    #[test]
    fn cache_roundtrip_format() {
        let tickets = vec![sample_ticket("X-1", "s", "Done")];
        let dir = std::env::temp_dir().join(format!("tick-app-cache-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let cache = crate::cache::ViewCache { dir: dir.clone() };
        cache.save_view(ViewMode::MyIssues, &tickets);
        let loaded = cache.load_view(ViewMode::MyIssues).unwrap();
        assert_eq!(loaded[0].key, "X-1");
        let _ = std::fs::remove_dir_all(dir);
    }
}
