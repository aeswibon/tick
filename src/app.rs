use crate::api::types::Ticket;
use crate::api::{self, JiraClient};
use crate::cache::ViewCache;
use crate::columns::Column;
use crate::config::{Config, SiteLinkTypes};
use crate::fetch_status::FetchStatus;
use crate::issue_key::{host_from_url, parse_issue_key};
use crate::platform;
use crate::theme::Theme;
use crate::ticket_lock::{read_tickets, write_tickets};
use crate::view_mode::ViewMode;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;

use chrono::{DateTime, Utc};
use crossterm::event::KeyEvent;

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
    Links,
}

impl DetailTab {
    pub fn next(self) -> Self {
        match self {
            Self::Details => Self::Description,
            Self::Description => Self::Comments,
            Self::Comments => Self::Links,
            Self::Links => Self::Details,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Details => Self::Links,
            Self::Description => Self::Details,
            Self::Comments => Self::Description,
            Self::Links => Self::Comments,
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
    EditDueDate,
    OpenTicket,
    /// Required workflow field (free text) while changing status.
    TransitionField,
    /// Create issue: summary or required custom field entry.
    CreateField,
    /// Create issue: optional description (markdown).
    CreateDescription,
    /// Save issue as template: template name in footer.
    TemplateExportName,
    /// Closed tab: JQL text search in footer.
    ClosedSearchQuery,
    TemplateEditSummary,
    TemplateEditProject,
    TemplateEditIssueType,
    TemplateEditDescription,
    TemplateEditLabels,
    /// Bulk replace labels on marked table rows (comma-separated footer).
    BulkEditLabels,
    /// Add issue link: target KEY after picking link type.
    AddIssueLinkTarget,
    /// Create subtask under current issue (summary only).
    CreateSubtaskSummary,
    /// Search all cached views (`Ctrl+g`).
    GlobalSearchQuery,
    /// Configured custom field (`[[detail.editable_fields]]`, type text).
    EditCustomField,
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
    pub create_session: Option<crate::create_flow::CreateSession>,
    pub template_export: Option<crate::template_export_flow::TemplateExportSession>,
    pub template_manage: Option<crate::template_manage_flow::TemplateManageSession>,
    pub showing_create_picker: bool,
    pub showing_transition_field: bool,
    /// True when the modal is showing a footer text prompt (no option list).
    pub transition_field_text_mode: bool,
    /// User field: footer search with live results in `transition_field_options`.
    pub transition_field_user_search: bool,
    pub transition_field_options: Vec<(String, String)>,
    pub transition_field_selected: usize,
    pub transition_field_heading: String,
    pub transition_field_current: Option<crate::api::transition_fields::TransitionField>,
    /// Checkbox mode for components / fixVersions (Space toggles, Enter confirms).
    pub transition_multi_mode: bool,
    pub transition_multi_picked: Vec<bool>,
    /// Lazy-loaded links + subtasks for the selected issue (Links tab).
    pub issue_relations: Option<crate::api::issue_relations::IssueRelations>,
    pub issue_relations_key: Option<(String, String)>,
    /// Combined links + subtasks row index on Links tab.
    pub links_selected: usize,
    pub showing_add_link: bool,
    pub add_link_selected: usize,
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
    /// Closed tab: last JQL search words (server-side `text ~`).
    pub closed_search_query: String,
    /// Closed tab: `true` → `assignee was currentUser()` (ever assigned).
    pub closed_search_ever_assigned: bool,
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
    /// When set, table shows a `[[views.custom]]` JQL view instead of `active_view`.
    pub custom_view_index: Option<usize>,
    pub view_cache: HashMap<ViewMode, Vec<Ticket>>,
    pub custom_view_cache: HashMap<String, Vec<Ticket>>,
    pub custom_field_ids: Vec<String>,
    cache: ViewCache,
    pub pending_bg_update: Arc<Mutex<Option<BgResult>>>,
    filter_cache: RefCell<Option<FilterCache>>,
    /// Bulk table selection: `(site, key)`.
    pub bulk_marked: HashSet<(String, String)>,
    pub bulk_action: Option<crate::bulk::BulkAction>,
    /// Cached cross-view search (`g`).
    pub showing_global_search: bool,
    pub global_search_hits: Vec<crate::global_search::GlobalSearchHit>,
    pub global_search_selected: usize,
    /// Pick which `[[detail.editable_fields]]` entry to edit (`F`).
    pub showing_editable_field_picker: bool,
    pub editable_field_picker_selected: usize,
    /// Select-list overlay for configured option fields.
    pub showing_custom_field_select: bool,
    pub custom_field_select_options: Vec<String>,
    pub custom_field_select_selected: usize,
    pub custom_field_editing: Option<crate::config::EditableFieldConfig>,
    /// Loading description/comments for the open detail pane.
    pub detail_loading: bool,
    pub plugins: crate::plugins::PluginHost,
}

impl App {
    pub fn new(config: Config, theme: Theme, jira: Arc<JiraClient>, debug: bool) -> Self {
        let cache = ViewCache::open();
        let columns = Column::resolve(config.columns.as_deref());
        let custom_field_ids = config.custom_field_ids_for_fetch();
        let page_size = config.page_size as usize;
        let closed_prefs = cache.load_closed_prefs();

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
            create_session: None,
            template_export: None,
            template_manage: None,
            showing_create_picker: false,
            showing_transition_field: false,
            transition_field_text_mode: false,
            transition_field_user_search: false,
            transition_field_options: Vec::new(),
            transition_field_selected: 0,
            transition_field_heading: String::new(),
            transition_field_current: None,
            transition_multi_mode: false,
            transition_multi_picked: Vec::new(),
            issue_relations: None,
            issue_relations_key: None,
            links_selected: 0,
            showing_add_link: false,
            add_link_selected: 0,
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
            closed_search_query: closed_prefs.query,
            closed_search_ever_assigned: closed_prefs.ever_assigned,
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
            custom_view_index: None,
            view_cache: HashMap::new(),
            custom_view_cache: HashMap::new(),
            custom_field_ids,
            cache,
            pending_bg_update: Arc::new(Mutex::new(None)),
            filter_cache: RefCell::new(None),
            bulk_marked: HashSet::new(),
            bulk_action: None,
            showing_global_search: false,
            global_search_hits: Vec::new(),
            global_search_selected: 0,
            showing_editable_field_picker: false,
            editable_field_picker_selected: 0,
            showing_custom_field_select: false,
            custom_field_select_options: Vec::new(),
            custom_field_select_selected: 0,
            custom_field_editing: None,
            detail_loading: false,
            plugins: crate::plugins::PluginHost::load(),
        };
        for err in &app.plugins.load_errors {
            eprintln!("[tick plugin] {err}");
        }
        app.load_cache();
        app
    }

    pub fn bulk_mark_count(&self) -> usize {
        self.bulk_marked.len()
    }

    pub fn clear_bulk_marks(&mut self) {
        self.bulk_marked.clear();
        self.bulk_action = None;
    }

    pub fn bulk_same_site(&self) -> Option<String> {
        let mut sites = self.bulk_marked.iter().map(|(s, _)| s.as_str());
        let first = sites.next()?;
        if sites.all(|s| s == first) {
            Some(first.to_string())
        } else {
            None
        }
    }

    /// Returns `true` when the issue was newly marked, `false` when unmarked.
    pub fn toggle_bulk_mark(&mut self, site: &str, key: &str) -> Result<bool, String> {
        let id = (site.to_string(), key.to_string());
        if self.bulk_marked.contains(&id) {
            self.bulk_marked.remove(&id);
            return Ok(false);
        }
        if self.bulk_marked.len() >= crate::bulk::BULK_MAX_SELECTED {
            return Err(format!(
                "Bulk selection limited to {} issues",
                crate::bulk::BULK_MAX_SELECTED
            ));
        }
        self.bulk_marked.insert(id);
        Ok(true)
    }

    pub fn bulk_marked_refs_in_filter_order(&self) -> Vec<TicketRef> {
        let tickets = read_tickets(&self.tickets);
        self.filtered_indices()
            .iter()
            .filter_map(|&idx| {
                let t = &tickets[idx];
                let id = (t.site.clone(), t.key.clone());
                self.bulk_marked.contains(&id).then(|| TicketRef {
                    site: t.site.clone(),
                    key: t.key.clone(),
                    link: t.link.clone(),
                })
            })
            .collect()
    }

    pub fn prune_bulk_marks(&mut self) {
        let tickets = read_tickets(&self.tickets);
        self.bulk_marked
            .retain(|(site, key)| tickets.iter().any(|t| &t.site == site && &t.key == key));
    }

    pub fn invalidate_filter_cache(&self) {
        *self.filter_cache.borrow_mut() = None;
    }

    pub fn is_custom_view_active(&self) -> bool {
        self.custom_view_index.is_some()
    }

    pub fn active_custom_view(&self) -> Option<&crate::config::CustomView> {
        self.custom_view_index
            .and_then(|i| self.config.views.custom.get(i))
    }

    pub fn save_closed_prefs(&self) {
        self.cache.save_closed_prefs(&crate::cache::ClosedPrefs {
            query: self.closed_search_query.clone(),
            ever_assigned: self.closed_search_ever_assigned,
        });
    }

    /// `(jql, optional site name filter)`.
    pub fn jql_for_current_fetch(&self) -> (String, Option<String>) {
        if let Some(view) = self.active_custom_view() {
            return (view.jql.clone(), view.site.clone());
        }
        if self.active_view == ViewMode::ClosedSearch {
            return (
                self.config.build_closed_search_jql(
                    &self.closed_search_query,
                    self.closed_search_ever_assigned,
                ),
                None,
            );
        }
        (self.config.jql_for(self.active_view).to_string(), None)
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

    /// Reload `config.toml` from disk (sites, views, templates, columns). Keeps current tickets.
    pub async fn reload_config(&mut self, debug: bool) -> Result<(), String> {
        let mut config = Config::load()?;
        let max_results = self.config.max_results;
        let page_size = self.config.page_size;
        config.apply_cli_overrides(Some(max_results), Some(page_size))?;

        let theme_name = config.theme.clone();
        self.theme = Theme::resolve(&theme_name)?;

        self.jira = Arc::new(
            api::JiraClient::from_config(&config, debug)
                .await
                .map_err(|e| format!("Auth after reload: {e}"))?,
        );

        self.config = config;
        self.columns = Column::resolve(self.config.columns.as_deref());
        self.custom_field_ids = self.config.custom_field_ids_for_fetch();
        self.page_size = self.config.page_size as usize;
        self.plugins = crate::plugins::PluginHost::load();
        for err in &self.plugins.load_errors {
            eprintln!("[tick plugin] {err}");
        }
        self.invalidate_filter_cache();
        self.status
            .set_action_notice("Config reloaded — press r to refresh views");

        let config_path = Config::config_path()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
        let findings = crate::config_check::validate_config(&self.config);
        crate::hooks::fire_on_config_reload(&self.config, &config_path, &findings);
        Ok(())
    }

    fn load_cache(&mut self) {
        self.view_cache = self.cache.load_all();
        if let Some(cached) = self.view_cache.get(&self.active_view).cloned() {
            self.install_ticket_list(cached);
            self.loading = false;
            self.live_data = false;
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

    pub fn invalidate_issue_relations_cache(&mut self) {
        self.issue_relations_key = None;
        self.links_selected = 0;
    }

    pub fn links_row_count(&self) -> usize {
        self.issue_relations
            .as_ref()
            .map(|r| r.combined_len())
            .unwrap_or(0)
    }

    pub fn clamp_links_selection(&mut self) {
        let max = self.links_row_count().saturating_sub(1);
        if self.links_selected > max {
            self.links_selected = max;
        }
    }

    /// Key of the selected row on the Links tab, if any.
    pub fn selected_link_key(&self) -> Option<String> {
        let rel = self.issue_relations.as_ref()?;
        rel.key_at(self.links_selected).map(str::to_string)
    }

    pub fn selected_link_id(&self) -> Option<String> {
        let rel = self.issue_relations.as_ref()?;
        rel.link_id_at(self.links_selected).map(str::to_string)
    }

    pub fn links_selection_is_link_row(&self) -> bool {
        self.issue_relations
            .as_ref()
            .is_some_and(|r| self.links_selected < r.links.len())
    }

    pub fn add_link_options(&self) -> Vec<(String, String)> {
        let Some(sel) = self.selected_ticket() else {
            return SiteLinkTypes::default().picker_options();
        };
        self.config
            .sites
            .iter()
            .find(|s| s.name == sel.site)
            .map(|s| s.link_types.picker_options())
            .unwrap_or_else(|| SiteLinkTypes::default().picker_options())
    }

    /// Select a row in the filtered table by issue key. Returns whether a match was found.
    pub fn try_select_ticket_by_key(&mut self, key: &str) -> bool {
        self.invalidate_filter_cache();
        let indices = self.filtered_indices();
        let tickets = match self.tickets.read() {
            Ok(g) => g,
            Err(_) => return false,
        };
        let found = indices.iter().position(|&ticket_idx| {
            tickets
                .get(ticket_idx)
                .is_some_and(|t| t.key.eq_ignore_ascii_case(key))
        });
        drop(tickets);
        if let Some(view_idx) = found {
            self.selected = view_idx;
            self.ensure_selection_visible();
            self.invalidate_issue_relations_cache();
            true
        } else {
            false
        }
    }

    pub fn select_ticket_by_key(&mut self, key: &str) {
        self.try_select_ticket_by_key(key);
    }

    pub fn move_selection_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.ensure_selection_visible();
            self.invalidate_issue_relations_cache();
        }
    }

    pub fn move_selection_down(&mut self) {
        let count = self.filtered_count();
        if count > 0 && self.selected + 1 < count {
            self.selected += 1;
            self.ensure_selection_visible();
            self.invalidate_issue_relations_cache();
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

    pub(crate) fn ensure_selection_visible(&mut self) {
        let viewport = self.table_viewport_rows.max(1);
        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        } else if self.selected >= self.scroll_offset + viewport {
            self.scroll_offset = self.selected + 1 - viewport;
        }
        self.clamp_scroll_offset();
    }

    fn apply_plugin_filters(&mut self, tickets: &mut Vec<Ticket>) {
        if let Err(e) = self.plugins.filter_tickets(tickets) {
            self.status
                .set_action_notice(format!("Plugin filter failed: {e}"));
        }
    }

    fn install_ticket_list(&mut self, mut tickets: Vec<Ticket>) {
        self.apply_plugin_filters(&mut tickets);
        write_tickets(&self.tickets).clone_from(&tickets);
        self.invalidate_filter_cache();
    }

    fn apply_fetch_result(
        &mut self,
        mut tickets: Vec<Ticket>,
        errors: Vec<String>,
        reset_cursor: bool,
    ) {
        self.apply_plugin_filters(&mut tickets);
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
        self.prune_bulk_marks();
        self.invalidate_filter_cache();
        self.live_data = !tickets.is_empty() || no_errors;
        if reset_cursor {
            self.scroll_offset = 0;
            self.selected = 0;
        }
    }

    pub async fn refresh(&mut self) {
        if self.is_custom_view_active() {
            self.refresh_custom_view().await;
            return;
        }
        if self.active_view == ViewMode::ClosedSearch && self.closed_search_query.trim().is_empty()
        {
            self.status.set_action_error(
                "Closed tab: press / to search done tickets (h = ever-assigned history)",
            );
            return;
        }
        self.loading = true;
        self.loading_message = Some("Fetching…".into());
        let (jql, site_filter) = self.jql_for_current_fetch();
        let (tickets, errors) = api::fetch_all(
            &self.jira,
            &self.config,
            &jql,
            site_filter.as_deref(),
            &self.custom_field_ids,
            &mut self.loading_message,
        )
        .await;
        self.view_cache.insert(self.active_view, tickets.clone());
        self.save_cache(self.active_view);
        let hooks_ok = errors.is_empty();
        self.apply_fetch_result(tickets, errors, true);
        self.loading = false;
        self.loading_message = None;
        self.last_refresh = Instant::now();
        self.view_fetched_at = Some(Utc::now());
        if hooks_ok {
            self.fire_refresh_hooks();
        }
        if self.detail_open {
            self.ensure_selected_issue_detail().await;
        }
    }

    pub async fn refresh_custom_view(&mut self) {
        let Some(view) = self.active_custom_view().cloned() else {
            return;
        };
        self.loading = true;
        self.loading_message = Some("Fetching…".into());
        let (tickets, errors) = api::fetch_all(
            &self.jira,
            &self.config,
            &view.jql,
            view.site.as_deref(),
            &self.custom_field_ids,
            &mut self.loading_message,
        )
        .await;
        let slug = view.cache_slug();
        self.custom_view_cache.insert(slug.clone(), tickets.clone());
        self.cache.save_custom_view(&slug, &tickets);
        let hooks_ok = errors.is_empty();
        self.apply_fetch_result(tickets, errors, true);
        self.loading = false;
        self.loading_message = None;
        self.last_refresh = Instant::now();
        self.view_fetched_at = Some(Utc::now());
        if hooks_ok {
            self.fire_refresh_hooks();
        }
        if self.detail_open {
            self.ensure_selected_issue_detail().await;
        }
    }

    pub async fn refresh_all(&mut self) {
        self.do_refresh_all(&ViewMode::background(), false, true)
            .await;
    }

    pub async fn refresh_all_notify(&mut self) {
        self.do_refresh_all(&ViewMode::background(), true, false)
            .await;
    }

    /// Fetch description and comments when the detail pane is open (lazy detail).
    pub async fn ensure_selected_issue_detail(&mut self) {
        if !self.detail_open {
            return;
        }
        let Some(idx) = self.selected_ticket_index() else {
            return;
        };
        let (site, key, already_loaded) = {
            let tickets = crate::ticket_lock::read_tickets(&self.tickets);
            let t = &tickets[idx];
            (t.site.clone(), t.key.clone(), t.detail_loaded)
        };
        if already_loaded {
            return;
        }
        let Some(base_url) = self.site_base_url(&site) else {
            self.status.set_action_error("Unknown site for ticket");
            return;
        };
        self.detail_loading = true;
        match self.jira.fetch_issue_detail(&base_url, &key).await {
            Ok(fields) => {
                let mut tickets = crate::ticket_lock::write_tickets(&self.tickets);
                if let Some(t) = tickets.get_mut(idx) {
                    if t.key == key && t.site == site {
                        t.apply_detail_fields(&fields);
                    }
                }
            }
            Err(e) => self.status.set_action_error(e),
        }
        self.detail_loading = false;
    }

    /// Load issue links and subtasks for the current selection (Links tab).
    pub async fn refresh_issue_relations(&mut self) {
        let Some(sel) = self.selected_ticket() else {
            self.issue_relations = None;
            self.issue_relations_key = None;
            return;
        };
        let cache_key = (sel.site.clone(), sel.key.clone());
        if self.issue_relations_key.as_ref() == Some(&cache_key) && self.issue_relations.is_some() {
            return;
        }
        let Some(base_url) = self.site_base_url(&sel.site) else {
            return;
        };
        self.loading = true;
        self.loading_message = Some(format!("Loading links for {}…", sel.key));
        match self.jira.fetch_issue_relations(&base_url, &sel.key).await {
            Ok(rel) => {
                self.issue_relations = Some(rel);
                self.issue_relations_key = Some(cache_key);
                self.clamp_links_selection();
            }
            Err(e) => {
                self.issue_relations = None;
                self.issue_relations_key = None;
                self.status.set_action_error(e);
            }
        }
        self.loading = false;
        self.loading_message = None;
    }

    async fn fetch_views_parallel(
        &self,
        views: &[ViewMode],
    ) -> Vec<(ViewMode, Vec<Ticket>, Vec<String>)> {
        let mut set = tokio::task::JoinSet::new();
        let cf = self.custom_field_ids.clone();
        for &mode in views {
            let jira = Arc::clone(&self.jira);
            let config = self.config.clone();
            let cf = cf.clone();
            set.spawn(async move {
                let jql = config.jql_for(mode);
                let mut loading = None;
                let (tickets, errors) =
                    api::fetch_all(&jira, &config, jql, None, &cf, &mut loading).await;
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
            if !self.is_custom_view_active() && mode == self.active_view {
                active_tickets = Some(tickets);
            }
            all_errors.extend(errs);
        }
        if self.is_custom_view_active() {
            self.status.set_site_warnings(all_errors);
        } else if let Some(tickets) = active_tickets {
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
        let custom_field_ids = self.custom_field_ids.clone();
        let pending = self.pending_bg_update.clone();
        let views: Vec<ViewMode> = ViewMode::background().to_vec();
        tokio::spawn(async move {
            let mut set = tokio::task::JoinSet::new();
            for mode in views.iter().copied() {
                let jira = Arc::clone(&jira);
                let config = config.clone();
                let cf = custom_field_ids.clone();
                set.spawn(async move {
                    let mut loading = None;
                    let (tickets, errors) = api::fetch_all(
                        &jira,
                        &config,
                        config.jql_for(mode),
                        None,
                        &cf,
                        &mut loading,
                    )
                    .await;
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
                if !self.is_custom_view_active() && mode == self.active_view {
                    active_tickets = Some(tickets);
                }
                all_errors.extend(errs);
            }
            if self.is_custom_view_active() {
                self.status.set_site_warnings(all_errors);
            } else if let Some(tickets) = active_tickets {
                let selected_key = self.selected_ticket().map(|t| t.key);
                let preserve_ui = same_ticket_keys(&previous_keys, &tickets);
                let hooks_ok = all_errors.is_empty();
                self.apply_fetch_result(tickets, all_errors, !preserve_ui);
                if preserve_ui {
                    restore_selection_on_key(self, selected_key.as_deref());
                }
                self.maybe_notify_new_tickets(&previous_keys, &read_tickets(&self.tickets));
                self.view_fetched_at = Some(Utc::now());
                if hooks_ok {
                    self.fire_refresh_hooks();
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

    pub(crate) fn current_view_hook_id(&self) -> String {
        if let Some(view) = self.active_custom_view() {
            view.name.clone()
        } else {
            self.active_view.cache_key().to_string()
        }
    }

    fn fire_refresh_hooks(&self) {
        let view_id = self.current_view_hook_id();
        let tickets = read_tickets(&self.tickets);
        crate::hooks::fire_on_refresh(&self.config, &view_id, &tickets);
    }

    pub fn plugin_context(&self) -> crate::plugins::PluginContext {
        let tickets = read_tickets(&self.tickets);
        let filtered = self
            .filtered_indices()
            .iter()
            .filter_map(|&i| tickets.get(i))
            .map(crate::plugins::PluginTicket::from)
            .collect();
        let selected = self
            .selected_ticket_entry()
            .map(|t| crate::plugins::PluginSelection {
                key: t.key,
                site: t.site,
            });
        crate::plugins::PluginContext {
            view_name: self.current_view_hook_id(),
            view_mode: if self.is_custom_view_active() {
                "custom".into()
            } else {
                self.active_view.cache_key().to_string()
            },
            tickets: filtered,
            selected,
        }
    }

    pub(crate) fn base_url_for_issue_key(&self, key: &str) -> Result<String, String> {
        if let Some(t) = self.selected_ticket_entry() {
            if t.key == key {
                return self
                    .site_base_url(&t.site)
                    .ok_or_else(|| format!("unknown site {:?}", t.site));
            }
        }
        let tickets = read_tickets(&self.tickets);
        if let Some(t) = tickets.iter().find(|t| t.key == key) {
            return self
                .site_base_url(&t.site)
                .ok_or_else(|| format!("unknown site {:?}", t.site));
        }
        if self.config.sites.len() == 1 {
            return Ok(self.config.sites[0]
                .base_url
                .trim_end_matches('/')
                .to_string());
        }
        Err(format!(
            "issue {key} not in current view; select it in the table first"
        ))
    }

    pub async fn plugin_list_transitions(
        &self,
        key: &str,
    ) -> Result<Vec<crate::operations::transition::TransitionSummary>, String> {
        let base_url = self.base_url_for_issue_key(key)?;
        crate::operations::transition::list_transitions(&self.jira, &base_url, key).await
    }

    pub async fn plugin_run_transition(
        &mut self,
        key: &str,
        transition_id: &str,
    ) -> Result<(), String> {
        let base_url = self.base_url_for_issue_key(key)?;
        crate::operations::transition::apply_transition_by_id(
            &self.jira,
            &base_url,
            key,
            transition_id,
        )
        .await?;
        self.refresh().await;
        Ok(())
    }

    /// Returns `true` when a plugin consumed the key (skip default handling).
    pub fn try_plugin_key(&mut self, key: &KeyEvent) -> bool {
        let ctx = self.plugin_context();
        let plugins = &self.plugins as *const crate::plugins::PluginHost;
        let app = self as *mut App;
        // SAFETY: `plugins` and the rest of `App` are distinct fields.
        match unsafe { (*plugins).try_handle_key(&ctx, &mut *app, key) } {
            Ok(crate::plugins::PluginKeyResult::Handled) => true,
            Ok(crate::plugins::PluginKeyResult::HandledWithNotice(msg)) => {
                self.status.set_action_notice(msg);
                true
            }
            Ok(crate::plugins::PluginKeyResult::Passthrough) => false,
            Err(e) => {
                self.status
                    .set_action_notice(format!("Plugin key failed: {e}"));
                false
            }
        }
    }

    pub async fn switch_to_custom(&mut self, index: usize) {
        if index >= self.config.views.custom.len() {
            return;
        }
        self.custom_view_index = Some(index);
        self.active_view = ViewMode::MyIssues;
        self.filter.clear();
        self.filtering = false;
        let view = &self.config.views.custom[index];
        let slug = view.cache_slug();
        if let Some(cached) = self.custom_view_cache.get(&slug).cloned() {
            self.install_ticket_list(cached);
            self.loading = false;
            self.live_data = false;
            self.scroll_offset = 0;
            self.selected = 0;
            self.detail_open = false;
            return;
        }
        if let Some(tickets) = self.cache.load_custom_view(&slug) {
            self.install_ticket_list(tickets.clone());
            self.custom_view_cache.insert(slug, tickets);
            self.loading = false;
            self.live_data = false;
            self.invalidate_filter_cache();
            self.scroll_offset = 0;
            self.selected = 0;
            self.detail_open = false;
            return;
        }
        self.refresh_custom_view().await;
        self.detail_open = false;
    }

    pub async fn cycle_custom_view(&mut self, next: bool) {
        let n = self.config.views.custom.len();
        if n == 0 {
            return;
        }
        let idx = match self.custom_view_index {
            Some(i) if next => (i + 1) % n,
            Some(i) => (i + n - 1) % n,
            None => 0,
        };
        self.switch_to_custom(idx).await;
    }

    pub async fn switch_to(&mut self, mode: ViewMode) {
        self.custom_view_index = None;
        self.active_view = mode;
        if let Some(cached) = self.view_cache.get(&mode).cloned() {
            self.install_ticket_list(cached);
            self.loading = false;
            self.live_data = false;
            self.scroll_offset = 0;
            self.selected = 0;
            self.sync_view_fetched_at(mode);
        } else if let Some(tickets) = self.cache.load_view(mode) {
            self.install_ticket_list(tickets.clone());
            self.view_cache.insert(mode, tickets);
            self.loading = false;
            self.live_data = false;
            self.scroll_offset = 0;
            self.selected = 0;
            self.sync_view_fetched_at(mode);
            self.detail_open = false;
            return;
        } else if mode == ViewMode::ClosedSearch && self.closed_search_query.trim().is_empty() {
            write_tickets(&self.tickets).clear();
            self.loading = false;
            self.live_data = false;
            self.invalidate_filter_cache();
            self.scroll_offset = 0;
            self.selected = 0;
            self.sync_view_fetched_at(mode);
        } else {
            self.refresh().await;
        }
        self.detail_open = false;
    }

    pub async fn refresh_closed_search(&mut self) {
        if self.closed_search_query.trim().is_empty() {
            self.status.set_action_error("Enter search words first (/)");
            return;
        }
        self.custom_view_index = None;
        self.active_view = ViewMode::ClosedSearch;
        self.save_closed_prefs();
        self.refresh().await;
    }

    pub fn toggle_closed_search_history(&mut self) {
        self.closed_search_ever_assigned = !self.closed_search_ever_assigned;
        self.save_closed_prefs();
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

/// Exposed for `cargo bench` (ticket filter + sort hot path).
#[doc(hidden)]
pub fn compute_filtered_indices_bench(
    tickets: &[Ticket],
    filter: &str,
    sort_mode: SortMode,
    sort_order: SortOrder,
) -> Vec<usize> {
    compute_filtered_indices(tickets, filter, sort_mode, sort_order)
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
                    || t.custom_fields
                        .values()
                        .any(|v| v.to_lowercase().contains(&q))
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
                ..Default::default()
            }],
            columns: None,
            max_results: 50,
            page_size,
            theme: "default".into(),
            views: Default::default(),
            notify_on_refresh: false,
            auth: Default::default(),
            oauth: Default::default(),
            create: Default::default(),
            hooks: Default::default(),
            detail: Default::default(),
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
            custom_fields: std::collections::HashMap::new(),
            detail_loaded: false,
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
            ..Default::default()
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
    fn invalidate_issue_relations_cache_clears_key_and_selection() {
        let mut app = App::new(test_config(20), Theme::default(), test_jira(), false);
        app.issue_relations_key = Some(("acme".into(), "DEMO-1".into()));
        app.issue_relations = Some(crate::api::issue_relations::IssueRelations::default());
        app.links_selected = 2;
        app.invalidate_issue_relations_cache();
        assert!(app.issue_relations_key.is_none());
        assert_eq!(app.links_selected, 0);
    }

    #[test]
    fn clamp_links_selection_caps_to_last_row() {
        let mut app = App::new(test_config(20), Theme::default(), test_jira(), false);
        app.issue_relations = Some(crate::api::issue_relations::IssueRelations {
            links: vec![],
            subtasks: vec![
                crate::api::issue_relations::SubtaskView {
                    key: "D-1".into(),
                    summary: "a".into(),
                    status: "Open".into(),
                },
                crate::api::issue_relations::SubtaskView {
                    key: "D-2".into(),
                    summary: "b".into(),
                    status: "Open".into(),
                },
            ],
        });
        app.links_selected = 99;
        app.clamp_links_selection();
        assert_eq!(app.links_selected, 1);
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
