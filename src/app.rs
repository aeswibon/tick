use crate::api::types::Ticket;
use crate::api::{self, JiraClient};
use crate::cache::ViewCache;
use crate::columns::Column;
use crate::config::Config;
use crate::fetch_status::FetchStatus;
use crate::issue_key::{host_from_url, parse_issue_key};
use crate::platform;
use crate::theme::Theme;
use crate::ticket_lock::{read_tickets, write_tickets};
use crate::view_mode::ViewMode;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;

use chrono::{DateTime, Utc};

type BgResult = Vec<(ViewMode, Vec<Ticket>, Vec<String>)>;

#[derive(Debug, Clone)]
pub struct TicketRef {
    pub site: String,
    pub key: String,
    pub link: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortOrder {
    #[default]
    Asc,
    Desc,
}

impl SortOrder {
    pub fn toggle(self) -> Self {
        match self {
            Self::Asc => Self::Desc,
            Self::Desc => Self::Asc,
        }
    }

    pub fn suffix(self) -> &'static str {
        match self {
            Self::Asc => "↑",
            Self::Desc => "↓",
        }
    }
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

    pub fn display(self, order: SortOrder) -> String {
        if self == Self::Default {
            self.label().to_string()
        } else {
            format!("{} {}", self.label(), order.suffix())
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
    EditLabels,
    EditDescription,
    OpenTicket,
    /// Required workflow field (free text) while changing status.
    TransitionField,
}

/// Collecting values for a workflow transition before POST.
#[derive(Debug, Clone)]
pub struct TransitionCollect {
    pub transition: crate::api::types::WorkflowTransition,
    pub values: std::collections::HashMap<String, serde_json::Value>,
    pub pending: Vec<crate::api::transition_fields::TransitionField>,
}

struct FilterCache {
    filter: String,
    sort_mode: SortMode,
    sort_order: SortOrder,
    ticket_count: usize,
    indices: Vec<usize>,
}

pub struct App {
    pub tickets: Arc<RwLock<Vec<Ticket>>>,
    pub jira: Arc<JiraClient>,
    pub last_refresh: Instant,
    /// When the active view was last fetched to disk (shown while `live_data` is false).
    pub view_fetched_at: Option<DateTime<Utc>>,
    pub loading: bool,
    /// Shown in header/footer while `loading` (e.g. multi-site issue lookup).
    pub loading_message: Option<String>,
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
    pub transition_options: Vec<crate::api::types::WorkflowTransition>,
    pub transition_collect: Option<TransitionCollect>,
    pub showing_transition_field: bool,
    pub transition_field_options: Vec<(String, String)>,
    pub transition_field_selected: usize,
    pub transition_field_heading: String,
    pub transition_field_current: Option<crate::api::transition_fields::TransitionField>,
    pub showing_priorities: bool,
    pub priority_selected: usize,
    pub priority_options: Vec<(String, String)>,
    pub showing_sprints: bool,
    pub sprint_selected: usize,
    pub sprint_options: Vec<(String, String)>,
    pub show_site_errors: bool,
    pub site_error_scroll: usize,
    pub live_data: bool,
    pub filter: String,
    pub filtering: bool,
    pub sort_mode: SortMode,
    pub sort_order: SortOrder,
    pub input_mode: InputMode,
    pub input_buffer: String,
    /// `@` mention picker while composing a comment.
    pub showing_mention_picker: bool,
    pub mention_selected: usize,
    /// `(account_id, display_name)`
    pub mention_options: Vec<(String, String)>,
    /// Byte index of the active `@` in `input_buffer`.
    pub mention_anchor: Option<usize>,
    /// Resolved @mentions while composing comment or description: (`@Display Name`, account_id).
    pub input_mentions: Vec<(String, String)>,
    #[allow(dead_code)]
    pub debug: bool,
    /// Rows to jump when pressing `[` / `]` (from config `page_size`).
    pub page_size: usize,
    /// Index into the filtered ticket list (not the raw tickets vec).
    pub scroll_offset: usize,
    /// Viewport height in rows (set each frame from terminal size).
    pub table_viewport_rows: usize,
    pub active_view: ViewMode,
    pub view_cache: HashMap<ViewMode, Vec<Ticket>>,
    cache: ViewCache,
    pub pending_bg_update: Arc<Mutex<Option<BgResult>>>,
    filter_cache: RefCell<Option<FilterCache>>,
}

impl App {
    pub fn new(config: Config, theme: Theme, jira: Arc<JiraClient>, debug: bool) -> Self {
        let cache = ViewCache::open();
        let columns = Column::resolve(config.columns.as_deref());
        let page_size = config.page_size as usize;

        let mut app = Self {
            tickets: Arc::new(RwLock::new(Vec::new())),
            jira,
            last_refresh: Instant::now(),
            view_fetched_at: None,
            loading: true,
            loading_message: None,
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
            transition_collect: None,
            showing_transition_field: false,
            transition_field_options: Vec::new(),
            transition_field_selected: 0,
            transition_field_heading: String::new(),
            transition_field_current: None,
            showing_priorities: false,
            priority_selected: 0,
            priority_options: Vec::new(),
            showing_sprints: false,
            sprint_selected: 0,
            sprint_options: Vec::new(),
            show_site_errors: false,
            site_error_scroll: 0,
            live_data: false,
            filter: String::new(),
            filtering: false,
            sort_mode: SortMode::Default,
            sort_order: SortOrder::default(),
            input_mode: InputMode::None,
            input_buffer: String::new(),
            showing_mention_picker: false,
            mention_selected: 0,
            mention_options: Vec::new(),
            mention_anchor: None,
            input_mentions: Vec::new(),
            debug,
            page_size,
            scroll_offset: 0,
            table_viewport_rows: 20,
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

    /// Build a browse URL for pasted text (issue key or Jira `/browse/` URL).
    pub async fn resolve_ticket_url(&mut self, raw: &str) -> Result<String, String> {
        let key = parse_issue_key(raw)
            .ok_or_else(|| "Paste an issue key (e.g. PROJ-123) or Jira browse URL".to_string())?;
        if let Some(link) = read_tickets(&self.tickets)
            .iter()
            .find(|t| t.key.eq_ignore_ascii_case(&key))
            .map(|t| t.link.clone())
        {
            return Ok(link);
        }

        if let Some(host) = host_from_url(raw) {
            for site in &self.config.sites {
                if host_from_url(&site.base_url).as_deref() == Some(host.as_str()) {
                    let base = site.base_url.trim_end_matches('/');
                    return Ok(format!("{base}/browse/{key}"));
                }
            }
        }

        if self.config.sites.len() > 1 {
            let total = self.config.sites.len();
            for (i, site) in self.config.sites.iter().enumerate() {
                self.loading_message =
                    Some(format!("Checking {} ({}/{})…", site.name, i + 1, total));
                if self.jira.issue_exists(&site.base_url, &key).await {
                    self.loading_message = None;
                    let base = site.base_url.trim_end_matches('/');
                    return Ok(format!("{base}/browse/{key}"));
                }
            }
            self.loading_message = None;
            return Err(format!("Issue {key} not found on any configured site"));
        }

        let site = self
            .config
            .sites
            .first()
            .ok_or_else(|| "No sites in config".to_string())?;
        let base = site.base_url.trim_end_matches('/');
        Ok(format!("{base}/browse/{key}"))
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
        let ticket_idx = self.selected_ticket_index()?;
        tickets.get(ticket_idx).cloned()
    }

    pub fn selected_ticket_index(&self) -> Option<usize> {
        self.filtered_indices().get(self.selected).copied()
    }

    pub fn set_table_viewport(&mut self, rows: usize) {
        self.table_viewport_rows = rows.max(1);
        self.clamp_selection();
    }

    fn load_cache(&mut self) {
        self.view_cache = self.cache.load_all();
        if let Some(cached) = self.view_cache.get(&self.active_view).cloned() {
            write_tickets(&self.tickets).clone_from(&cached);
            self.loading = false;
            self.live_data = false;
            self.invalidate_filter_cache();
        }
        self.sync_view_fetched_at(self.active_view);
    }

    pub fn sync_view_fetched_at(&mut self, mode: ViewMode) {
        self.view_fetched_at = self.cache.fetched_at_for(mode);
    }

    pub fn cache_age_suffix(&self) -> String {
        if self.live_data || self.loading {
            return String::new();
        }
        self.view_fetched_at
            .map(|at| format!(" · {}", format_cache_age(at)))
            .unwrap_or_default()
    }

    pub fn refresh_status_label(&self) -> String {
        if self.loading {
            return self
                .loading_message
                .clone()
                .unwrap_or_else(|| "loading".into());
        }
        if self.live_data {
            let mins = self.last_refresh.elapsed().as_secs() / 60;
            return format!("live · refresh {mins}m ago");
        }
        let offline = self.status.has_warnings() && !read_tickets(&self.tickets).is_empty();
        let prefix = if offline { "offline" } else { "cached" };
        if let Some(at) = self.view_fetched_at {
            return format!("{prefix} · {}", format_cache_age(at));
        }
        prefix.into()
    }

    pub fn save_cache(&self, mode: ViewMode) {
        if let Some(tickets) = self.view_cache.get(&mode) {
            self.cache.save_view(mode, tickets);
        }
    }

    /// Ticket indices visible in the current table viewport (virtualized window).
    pub fn visible_indices(&self) -> Vec<usize> {
        let all = self.filtered_indices();
        let viewport = self.table_viewport_rows;
        if viewport == 0 || all.is_empty() {
            return Vec::new();
        }
        let start = self.scroll_offset.min(all.len());
        let end = (start + viewport).min(all.len());
        all[start..end].to_vec()
    }

    pub fn selected_viewport_row(&self) -> usize {
        self.selected.saturating_sub(self.scroll_offset)
    }

    pub fn move_selection_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.ensure_selection_visible();
        }
    }

    pub fn move_selection_down(&mut self) {
        let count = self.filtered_count();
        if count > 0 && self.selected + 1 < count {
            self.selected += 1;
            self.ensure_selection_visible();
        }
    }

    pub fn scroll_page_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(self.page_size);
        self.clamp_scroll_offset();
    }

    pub fn scroll_page_down(&mut self) {
        let count = self.filtered_count();
        let max_offset = count.saturating_sub(self.table_viewport_rows);
        self.scroll_offset = (self.scroll_offset + self.page_size).min(max_offset);
        self.clamp_scroll_offset();
    }

    pub fn go_to_first(&mut self) {
        self.selected = 0;
        self.scroll_offset = 0;
    }

    pub fn go_to_last(&mut self) {
        let count = self.filtered_count();
        if count == 0 {
            self.selected = 0;
            self.scroll_offset = 0;
            return;
        }
        self.selected = count - 1;
        self.ensure_selection_visible();
    }

    fn clamp_scroll_offset(&mut self) {
        let viewport = self.table_viewport_rows.max(1);
        let count = self.filtered_count();
        let max_offset = count.saturating_sub(viewport);
        if self.scroll_offset > max_offset {
            self.scroll_offset = max_offset;
        }
    }

    fn ensure_selection_visible(&mut self) {
        let viewport = self.table_viewport_rows.max(1);
        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        } else if self.selected >= self.scroll_offset + viewport {
            self.scroll_offset = self.selected + 1 - viewport;
        }
        self.clamp_scroll_offset();
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
            self.scroll_offset = 0;
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
        self.view_fetched_at = Some(Utc::now());
    }

    pub async fn refresh_all(&mut self) {
        self.do_refresh_all(&ViewMode::all(), false, true).await;
    }

    pub async fn refresh_all_notify(&mut self) {
        self.do_refresh_all(&ViewMode::all(), true, false).await;
    }

    async fn fetch_views_parallel(
        &self,
        views: &[ViewMode],
    ) -> Vec<(ViewMode, Vec<Ticket>, Vec<String>)> {
        let mut set = tokio::task::JoinSet::new();
        for &mode in views {
            let jira = Arc::clone(&self.jira);
            let config = self.config.clone();
            set.spawn(async move {
                let (tickets, errors) = api::fetch_all(&jira, &config, config.jql_for(mode)).await;
                (mode, tickets, errors)
            });
        }
        let mut results = Vec::new();
        while let Some(res) = set.join_next().await {
            if let Ok(item) = res {
                results.push(item);
            }
        }
        results
    }

    async fn do_refresh_all(&mut self, views: &[ViewMode], notify: bool, preserve_ui: bool) {
        let track_keys = preserve_ui || (notify && self.config.notify_on_refresh);
        let previous_keys = if track_keys {
            ticket_keys(&read_tickets(&self.tickets))
        } else {
            Vec::new()
        };
        let selected_key = if preserve_ui {
            self.selected_ticket().map(|t| t.key)
        } else {
            None
        };
        self.loading = true;
        let results = self.fetch_views_parallel(views).await;
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
            let reset = !(preserve_ui && same_ticket_keys(&previous_keys, &tickets));
            self.apply_fetch_result(tickets, all_errors, reset);
            if preserve_ui && !reset {
                restore_selection_on_key(self, selected_key.as_deref());
            }
            self.maybe_notify_new_tickets(&previous_keys, &read_tickets(&self.tickets));
            self.view_fetched_at = Some(Utc::now());
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
        let views: Vec<ViewMode> = ViewMode::all().to_vec();
        tokio::spawn(async move {
            let mut set = tokio::task::JoinSet::new();
            for mode in views.iter().copied() {
                let jira = Arc::clone(&jira);
                let config = config.clone();
                set.spawn(async move {
                    let (tickets, errors) =
                        api::fetch_all(&jira, &config, config.jql_for(mode)).await;
                    (mode, tickets, errors)
                });
            }
            let mut results = Vec::new();
            while let Some(res) = set.join_next().await {
                if let Ok(item) = res {
                    results.push(item);
                }
            }
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
                let selected_key = self.selected_ticket().map(|t| t.key);
                let preserve_ui = same_ticket_keys(&previous_keys, &tickets);
                self.apply_fetch_result(tickets, all_errors, !preserve_ui);
                if preserve_ui {
                    restore_selection_on_key(self, selected_key.as_deref());
                }
                self.maybe_notify_new_tickets(&previous_keys, &read_tickets(&self.tickets));
                self.view_fetched_at = Some(Utc::now());
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
            self.scroll_offset = 0;
            self.selected = 0;
            self.sync_view_fetched_at(mode);
        } else if let Some(tickets) = self.cache.load_view(mode) {
            write_tickets(&self.tickets).clone_from(&tickets);
            self.view_cache.insert(mode, tickets.clone());
            self.loading = false;
            self.live_data = false;
            self.invalidate_filter_cache();
            self.scroll_offset = 0;
            self.selected = 0;
            self.sync_view_fetched_at(mode);
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
                && cache.sort_order == self.sort_order
                && cache.ticket_count == ticket_count
            {
                return cache.indices.clone();
            }
        }

        let indices =
            compute_filtered_indices(&tickets, &self.filter, self.sort_mode, self.sort_order);
        drop(tickets);

        *self.filter_cache.borrow_mut() = Some(FilterCache {
            filter: self.filter.clone(),
            sort_mode: self.sort_mode,
            sort_order: self.sort_order,
            ticket_count,
            indices: indices.clone(),
        });

        indices
    }

    fn clamp_selection(&mut self) {
        let count = self.filtered_count();
        if count == 0 {
            self.selected = 0;
            self.scroll_offset = 0;
            return;
        }
        if self.selected >= count {
            self.selected = count - 1;
        }
        self.ensure_selection_visible();
    }

    pub fn filtered_count(&self) -> usize {
        let ticket_count = read_tickets(&self.tickets).len();
        if let Some(cache) = self.filter_cache.borrow().as_ref() {
            if cache.filter == self.filter
                && cache.sort_mode == self.sort_mode
                && cache.sort_order == self.sort_order
                && cache.ticket_count == ticket_count
            {
                return cache.indices.len();
            }
        }
        self.filtered_indices().len()
    }

    fn maybe_notify_new_tickets(&self, previous_keys: &[String], tickets: &[Ticket]) {
        if !self.config.notify_on_refresh {
            return;
        }
        let new = tickets_new_since(previous_keys, tickets);
        if new.is_empty() {
            return;
        }
        let view = self.active_view.label();
        let body = if new.len() == 1 {
            format!("{} — {}", new[0].key, new[0].summary)
        } else {
            format!("{} new issues in {}", new.len(), view)
        };
        platform::notify("tick", &body);
    }
}

fn ticket_keys(tickets: &[Ticket]) -> Vec<String> {
    tickets.iter().map(|t| t.key.clone()).collect()
}

pub(crate) fn parse_labels_input(input: &str) -> Vec<String> {
    input
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect()
}

fn format_cache_age(at: DateTime<Utc>) -> String {
    let mins = (Utc::now() - at).num_minutes();
    if mins < 1 {
        "just now".into()
    } else if mins < 60 {
        format!("{mins}m ago")
    } else if mins < 24 * 60 {
        format!("{}h ago", mins / 60)
    } else {
        format!("{}d ago", mins / (24 * 60))
    }
}

fn tickets_new_since<'a>(previous: &[String], tickets: &'a [Ticket]) -> Vec<&'a Ticket> {
    if previous.is_empty() {
        return Vec::new();
    }
    tickets
        .iter()
        .filter(|t| !previous.contains(&t.key))
        .collect()
}

fn restore_selection_on_key(app: &mut App, key: Option<&str>) {
    let Some(key) = key else {
        app.clamp_selection();
        return;
    };
    let indices = app.filtered_indices();
    let match_pos = {
        let tickets = read_tickets(&app.tickets);
        indices.iter().position(|&i| tickets[i].key == key)
    };
    if let Some(pos) = match_pos {
        app.selected = pos;
        app.ensure_selection_visible();
    } else {
        app.clamp_selection();
    }
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

fn compute_filtered_indices(
    tickets: &[Ticket],
    filter: &str,
    sort_mode: SortMode,
    sort_order: SortOrder,
) -> Vec<usize> {
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

    if sort_mode != SortMode::Default && sort_order == SortOrder::Desc {
        indices.reverse();
    }

    indices
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::types::Ticket;
    use crate::theme::Theme;
    use std::sync::Arc;

    fn test_jira() -> Arc<JiraClient> {
        Arc::new(JiraClient::new("a@b.com", "t", false))
    }

    fn test_config(page_size: u32) -> Config {
        Config {
            email: "a@b.com".into(),
            token: "t".into(),
            sites: vec![crate::config::Site {
                name: "acme".into(),
                base_url: "https://acme.atlassian.net".into(),
                sprint_field: None,
                board_id: None,
                boards: Default::default(),
            }],
            columns: None,
            max_results: 50,
            page_size,
            theme: "default".into(),
            views: Default::default(),
            notify_on_refresh: false,
            auth: Default::default(),
            oauth: Default::default(),
            view_jql: Config::build_view_jql(&Default::default()),
        }
    }

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
            project_key: String::new(),
        }
    }

    #[tokio::test]
    async fn resolve_ticket_url_from_key_single_site() {
        let mut app = App::new(test_config(20), Theme::default(), test_jira(), false);
        let url = app.resolve_ticket_url("demo-9").await.unwrap();
        assert_eq!(url, "https://acme.atlassian.net/browse/DEMO-9");
    }

    #[tokio::test]
    async fn resolve_ticket_url_uses_loaded_ticket_link() {
        let mut app = App::new(test_config(20), Theme::default(), test_jira(), false);
        *write_tickets(&app.tickets) = vec![sample_ticket("WEB-1", "x", "Open")];
        let url = app.resolve_ticket_url("WEB-1").await.unwrap();
        assert_eq!(url, "https://acme.atlassian.net/browse/WEB-1");
    }

    #[tokio::test]
    async fn resolve_ticket_url_from_browse_url_host() {
        let mut cfg = test_config(20);
        cfg.sites.push(crate::config::Site {
            name: "other".into(),
            base_url: "https://other.atlassian.net".into(),
            sprint_field: None,
            board_id: None,
            boards: Default::default(),
        });
        let mut app = App::new(cfg, Theme::default(), test_jira(), false);
        let url = app
            .resolve_ticket_url("https://other.atlassian.net/browse/OTH-2")
            .await
            .unwrap();
        assert_eq!(url, "https://other.atlassian.net/browse/OTH-2");
    }

    #[test]
    fn parse_labels_input_splits_and_trims() {
        assert_eq!(parse_labels_input("a, b , ,c"), vec!["a", "b", "c"]);
        assert!(parse_labels_input("  , ").is_empty());
    }

    #[test]
    fn filter_matches_labels() {
        let mut t1 = sample_ticket("A-1", "one", "Open");
        t1.labels = vec!["backend".into()];
        let t2 = sample_ticket("A-2", "two", "Open");
        let tickets = vec![t1, t2];
        let idx = compute_filtered_indices(&tickets, "backend", SortMode::Default, SortOrder::Asc);
        assert_eq!(idx, vec![0]);
    }

    #[test]
    fn sort_order_reverses_age() {
        let mut t0 = sample_ticket("A-1", "old", "Open");
        t0.ageing_days = 10;
        let mut t1 = sample_ticket("A-2", "new", "Open");
        t1.ageing_days = 1;
        let list = [t0, t1];
        let asc = compute_filtered_indices(&list, "", SortMode::Age, SortOrder::Asc);
        let desc = compute_filtered_indices(&list, "", SortMode::Age, SortOrder::Desc);
        assert_eq!(asc, vec![1, 0]);
        assert_eq!(desc, vec![0, 1]);
    }

    #[test]
    fn filter_matches_summary() {
        let tickets = vec![
            sample_ticket("A-1", "login bug", "Open"),
            sample_ticket("A-2", "billing", "Open"),
        ];
        let idx = compute_filtered_indices(&tickets, "login", SortMode::Default, SortOrder::Asc);
        assert_eq!(idx, vec![0]);
    }

    #[test]
    fn config_page_size_is_scroll_step() {
        let app = App::new(test_config(7), Theme::default(), test_jira(), false);
        assert_eq!(app.page_size, 7);
    }

    #[test]
    fn same_ticket_keys_detects_unchanged_set() {
        let keys = vec!["A-1".into(), "B-2".into()];
        let tickets = vec![
            sample_ticket("A-1", "s", "Open"),
            sample_ticket("B-2", "s", "Open"),
        ];
        assert!(same_ticket_keys(&keys, &tickets));
        assert!(!same_ticket_keys(
            &keys,
            &[sample_ticket("C-3", "s", "Open")]
        ));
    }

    #[test]
    fn tickets_new_since_skips_empty_baseline() {
        let prev = vec!["A-1".into()];
        let tickets = vec![
            sample_ticket("A-1", "one", "Open"),
            sample_ticket("A-2", "two", "Open"),
        ];
        let new = tickets_new_since(&prev, &tickets);
        assert_eq!(new.len(), 1);
        assert_eq!(new[0].key, "A-2");
        assert!(tickets_new_since(&[], &tickets).is_empty());
    }

    #[test]
    fn virtualized_viewport_slices_filtered_list() {
        let theme = Theme::default();
        let mut app = App::new(test_config(10), theme, test_jira(), false);
        *write_tickets(&app.tickets) = (0..5)
            .map(|i| sample_ticket(&format!("T-{i}"), "x", "Open"))
            .collect();
        app.invalidate_filter_cache();
        app.set_table_viewport(2);
        app.scroll_offset = 2;
        app.selected = 3;
        assert_eq!(app.visible_indices(), vec![2, 3]);
        assert_eq!(app.selected_viewport_row(), 1);
    }

    #[test]
    fn scroll_page_down_advances_offset() {
        let mut app = App::new(test_config(2), Theme::default(), test_jira(), false);
        *write_tickets(&app.tickets) = (0..6)
            .map(|i| sample_ticket(&format!("T-{i}"), "x", "Open"))
            .collect();
        app.invalidate_filter_cache();
        app.set_table_viewport(2);
        app.scroll_page_down();
        assert_eq!(app.scroll_offset, 2);
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
