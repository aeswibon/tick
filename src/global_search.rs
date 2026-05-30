//! Search across all cached view tickets (disk + in-memory).

use std::collections::HashSet;

use crate::api::types::Ticket;
use crate::app::App;
use crate::ticket_lock::read_tickets;
use crate::view_mode::ViewMode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GlobalSearchTarget {
    BuiltIn(ViewMode),
    Custom(usize),
}

#[derive(Debug, Clone)]
pub struct GlobalSearchHit {
    pub view_label: String,
    pub ticket: Ticket,
    pub target: GlobalSearchTarget,
}

pub fn refresh_hits(app: &App, query: &str) -> Vec<GlobalSearchHit> {
    let q = query.trim().to_lowercase();
    let pool = collect_cached_tickets(app);
    let mut hits: Vec<GlobalSearchHit> = pool
        .into_iter()
        .filter(|h| q.is_empty() || ticket_matches(&h.ticket, &q))
        .collect();
    hits.sort_by(|a, b| {
        a.ticket
            .key
            .cmp(&b.ticket.key)
            .then_with(|| a.view_label.cmp(&b.view_label))
    });
    hits.truncate(50);
    hits
}

fn ticket_matches(ticket: &Ticket, q: &str) -> bool {
    ticket.key.to_lowercase().contains(q)
        || ticket.summary.to_lowercase().contains(q)
        || ticket.labels.iter().any(|l| l.to_lowercase().contains(q))
}

fn collect_cached_tickets(app: &App) -> Vec<GlobalSearchHit> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();

    for mode in ViewMode::background() {
        if let Some(tickets) = app.view_cache.get(&mode) {
            let label = mode.label().to_string();
            for t in tickets {
                push_hit(
                    &mut seen,
                    &mut out,
                    label.clone(),
                    t.clone(),
                    GlobalSearchTarget::BuiltIn(mode),
                );
            }
        }
    }

    for (i, view) in app.config.views.custom.iter().enumerate() {
        let slug = view.cache_slug();
        if let Some(tickets) = app.custom_view_cache.get(&slug) {
            let label = view.name.clone();
            for t in tickets {
                push_hit(
                    &mut seen,
                    &mut out,
                    label.clone(),
                    t.clone(),
                    GlobalSearchTarget::Custom(i),
                );
            }
        }
    }

    let tickets = read_tickets(&app.tickets);
    let current_label = if let Some(v) = app.active_custom_view() {
        v.name.clone()
    } else {
        app.active_view.label().to_string()
    };
    let target = if let Some(i) = app.custom_view_index {
        GlobalSearchTarget::Custom(i)
    } else {
        GlobalSearchTarget::BuiltIn(app.active_view)
    };
    for t in tickets.iter() {
        push_hit(&mut seen, &mut out, current_label.clone(), t.clone(), target.clone());
    }

    out
}

fn push_hit(
    seen: &mut HashSet<(String, String)>,
    out: &mut Vec<GlobalSearchHit>,
    view_label: String,
    ticket: Ticket,
    target: GlobalSearchTarget,
) {
    let id = (ticket.site.clone(), ticket.key.clone());
    if seen.insert(id) {
        out.push(GlobalSearchHit {
            view_label,
            ticket,
            target,
        });
    }
}

pub async fn jump_to_hit(app: &mut App, hit: &GlobalSearchHit) {
    match hit.target {
        GlobalSearchTarget::BuiltIn(mode) => app.switch_to(mode).await,
        GlobalSearchTarget::Custom(i) => app.switch_to_custom(i).await,
    }
    let key = hit.ticket.key.clone();
    let indices = app.filtered_indices();
    let match_pos = {
        let tickets = read_tickets(&app.tickets);
        indices.iter().position(|&i| tickets[i].key == key)
    };
    if let Some(pos) = match_pos {
        app.selected = pos;
        app.ensure_selection_visible();
    }
    app.showing_global_search = false;
    app.global_search_hits.clear();
    app.global_search_selected = 0;
    app.input_mode = crate::app::InputMode::None;
    app.input_buffer.clear();
}
