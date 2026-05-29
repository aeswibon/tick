mod api;
mod app;
mod columns;
mod config;
mod fetch_status;
mod input;
mod platform;
mod theme;
mod ticket_lock;
mod ui;
mod view_mode;

use app::App;
use clap::Parser;
use config::Config;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, time::Duration};
use theme::Theme;
pub use view_mode::ViewMode;

#[derive(Parser)]
#[command(name = "tick", about = "Jira ticket dashboard for the terminal")]
struct Cli {
    #[arg(long)]
    init: bool,

    #[arg(long)]
    doctor: bool,

    #[arg(long)]
    debug: bool,

    #[arg(long)]
    theme: Option<String>,

    #[arg(long)]
    max_results: Option<u32>,
}

async fn run_doctor(config: &Config) {
    let client = api::JiraClient::new(&config.email, &config.token, false);
    let probe_jql = "assignee = currentUser() AND statusCategory != Done ORDER BY updated DESC";

    for site in &config.sites {
        println!("--- {} ({}) ---", site.name, site.base_url);

        match client.search_jql(&site.base_url, probe_jql, 3).await {
            Ok(ids) => println!("  JQL search: OK ({} issue ids)", ids.len()),
            Err(e) => println!("  JQL search: FAILED — {e}"),
        }

        match client.search_jql(&site.base_url, probe_jql, 2).await {
            Ok(ids) => match client.bulk_fetch(&site.base_url, &ids).await {
                Ok(issues) => println!(
                    "  Bulk fetch: OK ({} ids → {} issues)",
                    ids.len(),
                    issues.len()
                ),
                Err(e) => println!("  Bulk fetch: FAILED — {e}"),
            },
            Err(e) => println!("  Bulk fetch: skipped — {e}"),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    if cli.init {
        Config::create_default_config().map_err(|e| format!("Config error: {}", e))?;
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
    }

    print_banner().await;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let theme_name = cli.theme.clone().unwrap_or_else(|| config.theme.clone());
    let theme = match Theme::resolve(&theme_name) {
        Ok(t) => t,
        Err(e) => {
            disable_raw_mode().ok();
            return Err(e.into());
        }
    };

    let mut app = App::new(config, theme, cli.debug);
    let refresh_interval = Duration::from_secs(3 * 60 * 60);
    app.spawn_background_refresh();

    let mut should_quit = false;
    while !should_quit {
        app.apply_pending_updates();
        terminal.draw(|f| ui::draw::render(f, &app))?;

        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    should_quit = input::handle_key(&mut app, key.code).await;
                }
            }
        }

        if app.last_refresh.elapsed() >= refresh_interval {
            app.refresh_all().await;
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

async fn print_banner() {
    println!();
    println!("████████╗██╗ ██████╗██╗  ██╗");
    println!("╚══██╔══╝██║██╔════╝██║ ██╔╝");
    println!("   ██║   ██║██║     █████╔╝ ");
    println!("   ██║   ██║██║     ██╔═██╗ ");
    println!("   ██║   ██║╚██████╗██║  ██╗");
    println!("   ╚═╝   ╚═╝ ╚═════╝╚═╝  ╚═╝");
    println!();
    println!("  tick — Jira TUI");
    tokio::time::sleep(Duration::from_millis(600)).await;
    println!();
}
