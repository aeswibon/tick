pub mod api;
pub mod app;
pub mod cache;
pub mod columns;
pub mod config;
pub mod fetch_status;
pub mod input;
pub mod platform;
pub mod theme;
pub mod ticket_lock;
pub mod ui;
pub mod view_mode;

pub use view_mode::ViewMode;

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

#[derive(Parser)]
#[command(name = "tick", about = "Jira ticket dashboard for the terminal")]
pub struct Cli {
    #[arg(long)]
    pub init: bool,

    #[arg(long)]
    pub doctor: bool,

    #[arg(long)]
    pub debug: bool,

    #[arg(long)]
    pub theme: Option<String>,

    #[arg(long)]
    pub max_results: Option<u32>,

    #[arg(long)]
    pub page_size: Option<u32>,

    #[arg(long)]
    pub list_themes: bool,
}

pub async fn run_doctor(config: &Config) {
    let client = api::JiraClient::new(&config.email, &config.token, false);
    let probe_jql = "assignee = currentUser() AND statusCategory != Done ORDER BY updated DESC";

    for site in &config.sites {
        println!("--- {} ({}) ---", site.name, site.base_url);

        match client.search_jql(&site.base_url, probe_jql, 3).await {
            Ok(ids) => println!("  JQL search: OK ({} issue ids)", ids.len()),
            Err(e) => println!("  JQL search: FAILED — {e}"),
        }

        match client.search_jql(&site.base_url, probe_jql, 2).await {
            Ok(ids) => {
                let sf = site.sprint_field.as_deref();
                match client.bulk_fetch(&site.base_url, &ids, sf).await {
                    Ok(issues) => println!(
                        "  Bulk fetch: OK ({} ids → {} issues)",
                        ids.len(),
                        issues.len()
                    ),
                    Err(e) => println!("  Bulk fetch: FAILED — {e}"),
                }
            }
            Err(e) => println!("  Bulk fetch: skipped — {e}"),
        }

        match client.find_sprint_fields(&site.base_url).await {
            Ok(fields) if fields.is_empty() => {
                println!("  Sprint fields: none found (set sprint_field in config if needed)");
            }
            Ok(fields) => {
                println!("  Sprint fields (use id in config sprint_field):");
                for (id, name) in fields {
                    let configured = site.sprint_field.as_deref() == Some(id.as_str());
                    let mark = if configured { " *" } else { "" };
                    println!("    {id} — {name}{mark}");
                }
            }
            Err(e) => println!("  Sprint fields: FAILED — {e}"),
        }
    }
}

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
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

    if cli.list_themes {
        println!("Available themes:");
        for name in Theme::list_available() {
            let builtin = Theme::all_builtin().contains_key(name.as_str());
            let tag = if builtin { "built-in" } else { "custom" };
            println!("  {name} ({tag})");
        }
        println!("\nSet theme in config.toml or pass --theme <name>");
        println!("Custom themes: ~/.config/tick/themes/<name>.toml");
        println!("Examples: themes/ in the tick repository");
        return Ok(());
    }

    if let Err(e) = config.apply_cli_overrides(cli.max_results, cli.page_size) {
        eprintln!("Config error: {}", e);
        std::process::exit(1);
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
        if app.apply_pending_updates() {
            app.spawn_background_refresh();
        }
        terminal.draw(|f| ui::draw::render(f, &mut app))?;

        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    should_quit = input::handle_key(&mut app, key.code).await;
                }
            }
        }

        if app.last_refresh.elapsed() >= refresh_interval {
            app.refresh_all_notify().await;
            app.spawn_background_refresh();
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
