use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tick::api::types::Ticket;
use tick::app::{compute_filtered_indices_bench, SortMode, SortOrder};
use tick::issue_key::parse_issue_key;
use tick::theme::Theme;
use tick::view_mode::{build_closed_search_jql, ViewMode};

fn sample_tickets(n: usize) -> Vec<Ticket> {
    (0..n)
        .map(|i| Ticket {
            key: format!("DEMO-{i}"),
            site: "acme".into(),
            issue_type: "Task".into(),
            status: "Open".into(),
            status_color: "yellow".into(),
            priority: "Medium".into(),
            ageing_days: i as i64,
            due_date: None,
            assignee: "Alice".into(),
            reporter: "Bob".into(),
            summary: format!("Summary number {i} for filtering"),
            link: format!("https://acme.atlassian.net/browse/DEMO-{i}"),
            description: None,
            description_adf: None,
            latest_comment: None,
            all_comments: vec![],
            parent_key: None,
            parent_summary: None,
            labels: vec!["backend".into()],
            sprint_name: Some("Sprint 1".into()),
            project_key: "DEMO".into(),
            custom_fields: Default::default(),
        })
        .collect()
}

fn bench_jql(c: &mut Criterion) {
    let base = ViewMode::closed_search_base(true);
    c.bench_function("build_closed_search_jql", |b| {
        b.iter(|| black_box(build_closed_search_jql(base, "regression login \"quoted\"")));
    });
}

fn bench_filter(c: &mut Criterion) {
    let tickets = sample_tickets(500);
    c.bench_function("compute_filtered_indices_500", |b| {
        b.iter(|| {
            black_box(compute_filtered_indices_bench(
                &tickets,
                "regression",
                SortMode::Age,
                SortOrder::Asc,
            ))
        });
    });
}

fn bench_theme(c: &mut Criterion) {
    c.bench_function("theme_resolve_default", |b| {
        b.iter(|| black_box(Theme::resolve("default").unwrap()));
    });
    c.bench_function("theme_list_available", |b| {
        b.iter(|| black_box(Theme::list_available()));
    });
}

fn bench_issue_key(c: &mut Criterion) {
    c.bench_function("parse_issue_key_browse_url", |b| {
        b.iter(|| {
            black_box(parse_issue_key(
                "https://acme.atlassian.net/browse/DEMO-42?focusedCommentId=1",
            ))
        });
    });
}

criterion_group!(
    benches,
    bench_jql,
    bench_filter,
    bench_theme,
    bench_issue_key
);
criterion_main!(benches);
