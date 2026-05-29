mod api;
mod app;
mod columns;
mod config;
mod fetch_status;
mod platform;
mod theme;
mod ui;

use app::{App, ViewMode};
use clap::Parser;
use config::Config;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, style::Style, text::{Line, Span}, widgets::Paragraph, Terminal};
use std::{
    io,
    time::Duration,
};
use theme::Theme;

#[derive(Parser)]
#[command(name = "tick", about = "Jira ticket dashboard for the terminal")]
struct Cli {
    /// Initialize default config file and exit
    #[arg(long)]
    init: bool,

    /// Test API connectivity and exit
    #[arg(long)]
    doctor: bool,

    /// Print API request/response details to stderr
    #[arg(long)]
    debug: bool,

    /// Theme (name or path) — overrides config.toml theme
    #[arg(long)]
    theme: Option<String>,

    /// Max results per site
    #[arg(long)]
    max_results: Option<u32>,
}

async fn run_doctor(config: &Config) {
    let client = api::JiraClient::new(&config.email, &config.token, false);

    for site in &config.sites {
        println!("--- {} ({}) ---", site.name, site.base_url);

        let jql_url = format!("{}/rest/api/3/search/jql", site.base_url.trim_end_matches('/'));

        // Test 1: any issues at all (minimal bound) — print raw JSON
        match client.http.post(&jql_url).basic_auth(&client.email, Some(&client.token)).json(&serde_json::json!({"jql": "created > -30d ORDER BY updated DESC", "maxResults": 3})).send().await {
            Ok(resp) => {
                let status = resp.status();
                match resp.text().await {
                    Ok(body) => {
                        if status.is_success() {
                            println!("  Raw JSON response (site: {}):", site.name);
                            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&body) {
                                // Pretty-print the issues array to see all fields
                                if let Some(issues) = parsed.get("issues").and_then(|v| v.as_array()) {
                                    println!("  Number of issues: {}", issues.len());
                                    if let Some(first) = issues.first() {
                                        println!("  First issue fields: {:?}", serde_json::to_string_pretty(first).unwrap_or_default());
                                    }
                                }
                            }
                        } else {
                            println!("  StatusCategory test: FAILED ({}): {}", status, body);
                        }
                    }
                    Err(e) => println!("  StatusCategory test: ERROR reading response: {}", e),
                }
            }
            Err(e) => println!("  StatusCategory test: NETWORK ERROR: {}", e),
        }

        // Test 2: non-done issues assigned to currentUser
        let ids = match client.http.post(&jql_url).basic_auth(&client.email, Some(&client.token)).json(&serde_json::json!({"jql": "assignee = currentUser() AND statusCategory != Done ORDER BY updated DESC", "maxResults": 5})).send().await {
            Ok(resp) => {
                let status = resp.status();
                match resp.text().await {
                    Ok(body) => {
                        if status.is_success() {
                            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&body) {
                                let count = parsed.get("issues").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0);
                                println!("  My issues: {} issues (assignee = currentUser())", count);
                                let ids: Vec<String> = parsed.get("issues").and_then(|v| v.as_array()).into_iter().flatten()
                                    .filter_map(|i| i.get("id").and_then(|v| v.as_str()).map(String::from))
                                    .collect();
                                for id in &ids {
                                    println!("    - {}", id);
                                }
                                ids
                            } else {
                                vec![]
                            }
                        } else {
                            println!("  My issues: FAILED ({}): {}", status, body);
                            vec![]
                        }
                    }
                    Err(e) => {
                        println!("  My issues: ERROR reading response: {}", e);
                        vec![]
                    }
                }
            }
            Err(e) => {
                println!("  My issues: NETWORK ERROR: {}", e);
                vec![]
            }
        };

        // Test 3: bulk fetch with first 2 IDs
        if ids.len() >= 2 {
            let bf_url = format!("{}/rest/api/3/issue/bulkfetch", site.base_url.trim_end_matches('/'));
            match client.http.post(&bf_url).basic_auth(&client.email, Some(&client.token)).json(&serde_json::json!({
                "issueIdsOrKeys": &ids[..2],
                "fields": ["issuetype", "status", "priority", "assignee", "reporter", "summary"],
            })).send().await {
                Ok(resp) => {
                    let status = resp.status();
                    match resp.text().await {
                        Ok(body) => {
                            if status.is_success() {
                                println!("  Bulk fetch (2 issues): OK");
                                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&body) {
                                    println!("  Full response: {}", serde_json::to_string_pretty(&parsed).unwrap_or_default());
                                } else {
                                    println!("  Raw response: {}", &body[..body.len().min(3000)]);
                                }
                            } else {
                                println!("  Bulk fetch: FAILED ({}): {}", status, body);
                            }
                        }
                        Err(e) => println!("  Bulk fetch: ERROR reading response: {}", e),
                    }
                }
                Err(e) => println!("  Bulk fetch: NETWORK ERROR: {}", e),
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    if cli.init {
        Config::create_default_config()
            .map_err(|e| format!("Config error: {}", e))?;
        return Ok(());
    }

    let mut config = match Config::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Config error: {}", e);
            eprintln!("Run `tick --init` to create a default config file");
            std::process::exit(1);
        }
    };

    if cli.doctor {
        run_doctor(&config).await;
        return Ok(());
    }

    if let Some(mr) = cli.max_results {
        config.max_results = mr;
    }

    if cli.debug {
        eprintln!("[DEBUG] Config loaded with {} sites", config.sites.len());
        for site in &config.sites {
            eprintln!("[DEBUG]   Site: {} -> {}", site.name, site.base_url);
        }
    }

    println!();
    println!("████████╗██╗ ██████╗██╗  ██╗");
    println!("╚══██╔══╝██║██╔════╝██║ ██╔╝");
    println!("   ██║   ██║██║     █████╔╝ ");
    println!("   ██║   ██║██║     ██╔═██╗ ");
    println!("   ██║   ██║╚██████╗██║  ██╗");
    println!("   ╚═╝   ╚═╝ ╚═════╝╚═╝  ╚═╝");
    println!();
    println!("  tick — Jira TUI");
    tokio::time::sleep(std::time::Duration::from_millis(600)).await;
    println!();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let theme_name = cli.theme.clone().unwrap_or_else(|| config.theme.clone());
    let theme = match Theme::resolve(&theme_name) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };

    let mut app = App::new(config, theme, cli.debug);
    let refresh_interval = Duration::from_secs(3 * 60 * 60);
    app.spawn_background_refresh();

    loop {
        app.apply_pending_updates();

        terminal.draw(|f| {
            let chunks = ratatui::layout::Layout::default()
                .direction(ratatui::layout::Direction::Vertical)
                .constraints([
                    ratatui::layout::Constraint::Length(1),
                    ratatui::layout::Constraint::Length(1),
                    ratatui::layout::Constraint::Min(1),
                    ratatui::layout::Constraint::Length(1),
                    ratatui::layout::Constraint::Length(1),
                ])
                .split(f.area());

            let (header_area, tab_area, main_area, _footer_gap, footer_area) =
                (chunks[0], chunks[1], chunks[2], chunks[3], chunks[4]);

            // --- Header bar ---
            let count = app.filtered_count();
            let sites = app.sites_str();
            let elapsed = app.last_refresh.elapsed();
            let mins = elapsed.as_secs() / 60;
            let right_text = format!(" {} tickets | refresh {}m ago", count, mins);

            let header_chunks = ratatui::layout::Layout::default()
                .direction(ratatui::layout::Direction::Horizontal)
                .constraints([
                    ratatui::layout::Constraint::Min(1),
                    ratatui::layout::Constraint::Length(7),
                ])
                .split(header_area);

            f.render_widget(
                Paragraph::new(Line::from(Span::raw(
                    format!(" {} | {}", sites, right_text)
                )))
                .style(Style::default().bg(app.theme.header_bg).fg(app.theme.fg)),
                header_chunks[0],
            );
            f.render_widget(
                Paragraph::new(Line::from(Span::styled("[tick]", Style::default().fg(app.theme.tick_fg))))
                    .style(Style::default().bg(app.theme.header_bg)),
                header_chunks[1],
            );

            // --- View tabs ---
            let tabs = [ViewMode::MyIssues, ViewMode::Updated, ViewMode::Mentions, ViewMode::Watching];
            let mut tab_spans = Vec::new();
            for tab in &tabs {
                let is_active = *tab == app.active_view;
                let label = tab.label();
                let style = if is_active {
                    Style::default().fg(app.theme.accent).add_modifier(ratatui::style::Modifier::BOLD)
                } else {
                    Style::default().fg(app.theme.border)
                };
                tab_spans.push(Span::styled(
                    if is_active { format!(" [{}]", label) } else { format!("  {}", label) },
                    style,
                ));
            }
            f.render_widget(
                Paragraph::new(Line::from(tab_spans)),
                tab_area,
            );

            // --- Main content ---
            if app.detail_open {
                ui::detail::draw_detail(f, &app, main_area);
            } else {
                ui::table::draw_table(f, &app, main_area);
            }

            if app.show_help {
                ui::help::draw_help(f, &app, f.area());
            }

            if app.showing_transitions {
                ui::transitions::draw_transitions(f, &app, f.area());
            }

            // --- Footer bar ---
            let (footer_text, fg_color) = if app.filtering {
                (format!(" Filter: {}_", app.filter), app.theme.accent)
            } else if app.input_mode == app::InputMode::Comment {
                (format!(" Comment: {}_", app.input_buffer), app.theme.accent)
            } else if app.input_mode == app::InputMode::Worklog {
                (format!(" Worklog (e.g. 30m): {}_", app.input_buffer), app.theme.accent)
            } else if app.loading {
                (" Loading...".to_string(), app.theme.loading_fg)
            } else if let Some(ref err) = app.status.action_error {
                (format!(" Error: {}", err), app.theme.error_fg)
            } else if app.status.has_warnings() {
                (
                    format!(" Warning:{}", app.status.format_warnings(72)),
                    app.theme.status_yellow,
                )
            } else {
                let mut left = " ? help  / filter  j/k  s sort  y copy  t trans  c comment  w worklog  [ ] page  ←/→ view  q quit".to_string();
                if app.detail_open {
                    left.push_str("  h/l tabs");
                }
                let right = format!(" {} | Page {}/{} | Sort: {} | {} tickets",
                    app.active_view.label(), app.current_page + 1, app.total_pages(),
                    app.sort_mode.label(), app.filtered_count());
                (format!("{:<60}{}", left, right), app.theme.footer_fg)
            };

            f.render_widget(
                Paragraph::new(Line::from(Span::styled(footer_text, Style::default().fg(fg_color))))
                    .style(Style::default().bg(app.theme.footer_bg)),
                footer_area,
            );
        })?;

        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    if app.filtering {
                        match key.code {
                            KeyCode::Char(c) => app.filter.push(c),
                            KeyCode::Backspace => { app.filter.pop(); }
                            KeyCode::Esc | KeyCode::Enter => {
                                app.filtering = false;
                                app.selected = 0;
                            }
                            _ => {}
                        }
                    } else if app.input_mode != app::InputMode::None {
                        match key.code {
                            KeyCode::Char(c) => app.input_buffer.push(c),
                            KeyCode::Backspace => { app.input_buffer.pop(); }
                            KeyCode::Esc => { app.input_mode = app::InputMode::None; }
                            KeyCode::Enter => {
                                let buffer = app.input_buffer.clone();
                                let mode = app.input_mode;
                                app.input_mode = app::InputMode::None;
                                app.input_buffer.clear();

                                let (site_name, key) = {
                                    let tickets = app.tickets.read().unwrap();
                                    let indices = app.visible_indices();
                                    if app.selected < indices.len() {
                                        let t = &tickets[indices[app.selected]];
                                        (t.site.clone(), t.key.clone())
                                    } else {
                                        continue;
                                    }
                                };
                                let base_url = app.config.sites.iter()
                                    .find(|s| s.name == site_name)
                                    .map(|s| s.base_url.clone())
                                    .unwrap_or_default();

                                let client = api::JiraClient::new(&app.config.email, &app.config.token, app.debug);
                                match mode {
                                    app::InputMode::Comment => {
                                        match client.add_comment(&base_url, &key, &buffer).await {
                                            Ok(_) => { app.refresh().await; }
                                            Err(e) => app.status.set_action_error(e),
                                        }
                                    }
                                    app::InputMode::Worklog => {
                                        match client.add_worklog(&base_url, &key, &buffer).await {
                                            Ok(_) => { app.refresh().await; }
                                            Err(e) => app.status.set_action_error(e),
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            _ => {}
                        }
                    } else if app.showing_transitions {
                        match key.code {
                            KeyCode::Char(n) if n >= '1' && n <= '9' => {
                                let idx = (n as u8 - b'1') as usize;
                                if idx < app.transition_options.len() {
                                    let (trans_id, _) = app.transition_options[idx].clone();
                                    let (site_name, key) = {
                                        let tickets = app.tickets.read().unwrap();
                                        let indices = app.visible_indices();
                                        if app.selected < indices.len() {
                                            let t = &tickets[indices[app.selected]];
                                            (t.site.clone(), t.key.clone())
                                        } else {
                                            continue;
                                        }
                                    };
                                    let base_url = app.config.sites.iter()
                                        .find(|s| s.name == site_name)
                                        .map(|s| s.base_url.clone())
                                        .unwrap_or_default();
                                    let client = api::JiraClient::new(&app.config.email, &app.config.token, app.debug);
                                    let url = format!("{}/rest/api/3/issue/{}/transitions", base_url.trim_end_matches('/'), key);
                                    let resp = client.http
                                        .post(&url)
                                        .basic_auth(&client.email, Some(&client.token))
                                        .json(&serde_json::json!({
                                            "transition": { "id": trans_id }
                                        }))
                                        .send()
                                        .await;
                                    match resp {
                                        Ok(r) => {
                                            if r.status().is_success() {
                                                app.showing_transitions = false;
                                                app.refresh().await;
                                            } else {
                                                let body = r.text().await.unwrap_or_default();
                                                app.status.set_action_error(format!("Transition failed: {}", body));
                                                app.showing_transitions = false;
                                            }
                                        }
                                        Err(e) => {
                                            app.status.set_action_error(format!("HTTP error: {}", e));
                                            app.showing_transitions = false;
                                        }
                                    }
                                }
                            }
                            KeyCode::Esc => { app.showing_transitions = false; }
                            _ => {}
                        }
                    } else {
                        match key.code {
                            KeyCode::Char('q') => break,
                            KeyCode::Char('r') => { app.refresh().await; }
                            KeyCode::Up | KeyCode::Char('k') => {
                                if app.selected > 0 {
                                    app.selected -= 1;
                                } else {
                                    app.prev_page();
                                }
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                let visible_count = app.visible_indices().len();
                                if visible_count > 0 && app.selected + 1 < visible_count {
                                    app.selected += 1;
                                } else {
                                    app.next_page();
                                }
                            }
                            KeyCode::Char('[') => app.prev_page(),
                            KeyCode::Char(']') => app.next_page(),
                            KeyCode::Char('/') => {
                                app.filtering = true;
                                app.filter.clear();
                                app.detail_open = false;
                                app.current_page = 0;
                                app.selected = 0;
                            }
                            KeyCode::Enter => {
                                if app.show_help {
                                    app.show_help = false;
                                } else if !app.detail_open {
                                    app.detail_open = true;
                                }
                            }
                            KeyCode::Esc => {
                                app.show_help = false;
                                app.detail_open = false;
                                app.showing_transitions = false;
                            }
                            KeyCode::Char('?') => {
                                app.show_help = !app.show_help;
                                app.detail_open = false;
                            }
                            KeyCode::Char('s') => {
                                app.sort_mode = app.sort_mode.next();
                                app.current_page = 0;
                                app.selected = 0;
                            }
                            KeyCode::Char('h') => {
                                if app.detail_open {
                                    app.detail_tab = match app.detail_tab {
                                        app::DetailTab::Details => app::DetailTab::Comments,
                                        app::DetailTab::Description => app::DetailTab::Details,
                                        app::DetailTab::Comments => app::DetailTab::Description,
                                    };
                                }
                            }
                            KeyCode::Char('l') => {
                                if app.detail_open {
                                    app.detail_tab = match app.detail_tab {
                                        app::DetailTab::Details => app::DetailTab::Description,
                                        app::DetailTab::Description => app::DetailTab::Comments,
                                        app::DetailTab::Comments => app::DetailTab::Details,
                                    };
                                }
                            }
                            KeyCode::Right => {
                                app.switch_to(app.active_view.next()).await;
                            }
                            KeyCode::Left => {
                                app.switch_to(app.active_view.prev()).await;
                            }
                            KeyCode::Char('e') => {
                                if let Ok(path) = config::Config::config_path() {
                                    let _ = platform::open_path(&path);
                                }
                            }
                            KeyCode::Char('y') => {
                                let tickets = app.tickets.read().unwrap();
                                let indices = app.visible_indices();
                                if app.selected < indices.len() {
                                    let t = &tickets[indices[app.selected]];
                                    if !platform::copy_to_clipboard(&t.key) {
                                        app.status.set_action_error("Clipboard unavailable on this system");
                                    }
                                }
                            }
                            KeyCode::Char('o') => {
                                if !app.detail_open {
                                    let tickets = app.tickets.read().unwrap();
                                    let indices = app.visible_indices();
                                    if app.selected < indices.len() {
                                        let link = &tickets[indices[app.selected]].link;
                                        if platform::open_url(link).is_err() {
                                            app.status.set_action_error("Could not open browser");
                                        }
                                    }
                                }
                            }
                            KeyCode::Char('1') => app.switch_to(ViewMode::MyIssues).await,
                            KeyCode::Char('2') => app.switch_to(ViewMode::Updated).await,
                            KeyCode::Char('3') => app.switch_to(ViewMode::Mentions).await,
                            KeyCode::Char('4') => app.switch_to(ViewMode::Watching).await,
                            KeyCode::Char('t') => {
                                let tickets = app.tickets.read().unwrap();
                                let indices = app.visible_indices();
                                if app.selected < indices.len() {
                                    let t = &tickets[indices[app.selected]];
                                    let site_name = t.site.clone();
                                    let key = t.key.clone();
                                    let base_url = app.config.sites.iter()
                                        .find(|s| s.name == site_name)
                                        .map(|s| s.base_url.clone())
                                        .unwrap_or_default();
                                    if !base_url.is_empty() {
                                        let client = api::JiraClient::new(&app.config.email, &app.config.token, app.debug);
                                        match client.get_transition_options(&base_url, &key).await {
                                            Ok(options) => {
                                                if !options.is_empty() {
                                                    app.transition_options = options;
                                                    app.showing_transitions = true;
                                                } else {
                                                    app.status.set_action_error("No transitions available");
                                                }
                                            }
                                            Err(e) => app.status.set_action_error(e),
                                        }
                                    }
                                }
                            }
                            KeyCode::Char('c') => {
                                if app.detail_open {
                                    app.input_mode = app::InputMode::Comment;
                                    app.input_buffer.clear();
                                }
                            }
                            KeyCode::Char('w') => {
                                if app.detail_open {
                                    app.input_mode = app::InputMode::Worklog;
                                    app.input_buffer.clear();
                                }
                            }
                            KeyCode::Char('g') => { app.current_page = 0; app.selected = 0; }
                            KeyCode::Char('G') => { app.go_to_last(); }
                            KeyCode::Tab => {
                                app.switch_to(app.active_view.next()).await;
                            }
                            KeyCode::BackTab => {
                                app.switch_to(app.active_view.prev()).await;
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        if app.last_refresh.elapsed() >= refresh_interval {
            app.refresh_all().await;
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    Ok(())
}
