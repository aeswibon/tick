pub mod api;
pub mod app;
pub mod auth;
pub mod auth_status;
pub mod batch;
pub mod bulk;
pub mod cache;
pub mod cli;
pub mod columns;
pub mod config;
pub mod config_check;
pub mod create_flow;
pub mod editable_fields;
pub mod fetch_status;
pub mod global_search;
pub mod hooks;
pub mod input;
pub mod issue_key;
pub mod issue_relations_flow;
pub mod oauth;
pub mod operations;
pub mod platform;
pub mod template_export;
pub mod template_export_flow;
pub mod template_manage_flow;
pub mod template_persist;
pub mod theme;
pub mod ticket_lock;
pub mod ui;
pub mod view_mode;

pub use view_mode::ViewMode;

use app::App;
use clap::{Parser, Subcommand};
use config::Config;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind,
        KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, time::Duration};
use theme::Theme;

#[derive(Parser)]
#[command(name = "tick", about = "Jira ticket dashboard for the terminal")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<TickCommand>,

    #[arg(long, global = true)]
    pub init: bool,

    #[arg(long, global = true)]
    pub doctor: bool,

    /// Offline config validation (structural; use --doctor for live Jira probes)
    #[arg(long, global = true)]
    pub check: bool,

    #[arg(long, global = true)]
    pub debug: bool,

    #[arg(long, global = true)]
    pub theme: Option<String>,

    #[arg(long, global = true)]
    pub max_results: Option<u32>,

    #[arg(long, global = true)]
    pub page_size: Option<u32>,

    #[arg(long, global = true)]
    pub list_themes: bool,
}

#[derive(Subcommand)]
pub enum TickCommand {
    /// Auth login / status / logout (status covers API token and OAuth)
    Auth {
        #[command(subcommand)]
        action: AuthCommand,
    },
    /// Issue template utilities
    Template {
        #[command(subcommand)]
        action: TemplateCommand,
    },
    /// Headless issue operations (JSON output)
    Issue {
        #[command(subcommand)]
        action: cli::issue::IssueCommand,
    },
    /// Search issues via JQL (JSON)
    Search(cli::search::SearchArgs),
    /// Bulk operations on multiple issue keys
    Bulk {
        #[command(subcommand)]
        action: cli::bulk_cmd::BulkCommand,
    },
}

#[derive(Subcommand)]
pub enum TemplateCommand {
    /// Fetch issues and emit [[templates]] TOML for config
    Export(TemplateExportArgs),
}

#[derive(Parser)]
pub struct TemplateExportArgs {
    /// Site name from config ([[sites]].name)
    pub site: String,
    /// Issue keys to export (e.g. HIN-123)
    pub keys: Vec<String>,
    /// Write to file (append with --append)
    #[arg(short, long)]
    pub output: Option<std::path::PathBuf>,
    /// Append to an existing templates file
    #[arg(long)]
    pub append: bool,
}

#[derive(Subcommand)]
pub enum AuthCommand {
    /// Browser login; stores tokens in ~/.config/tick/oauth.json
    Login,
    /// Show auth status (API token and/or OAuth session)
    Status,
    /// Remove stored OAuth tokens
    Logout,
}

pub async fn run_doctor(config: &Config) {
    let client = match api::JiraClient::from_config(config, false).await {
        Ok(c) => c,
        Err(e) => {
            println!("Auth error: {e}");
            return;
        }
    };
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
                match client.bulk_fetch(&site.base_url, &ids, sf, &[], true).await {
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

        match client.list_boards(&site.base_url).await {
            Ok(boards) if boards.is_empty() => {
                println!("  Agile boards: none found (set board_id or boards in config)");
            }
            Ok(boards) => {
                println!("  Agile boards (board_id / boards.PROJ in config):");
                for board in boards {
                    let pk = board.project_key.as_deref();
                    let mark = if site.is_board_configured(board.id, pk) {
                        " *"
                    } else {
                        ""
                    };
                    let proj = pk.unwrap_or("-");
                    println!("    {} — {} ({}){mark}", board.id, board.name, proj);
                }
            }
            Err(e) => println!("  Agile boards: FAILED — {e}"),
        }
    }
}

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    if let Some(TickCommand::Auth { action }) = cli.command {
        return run_auth_command(action).await;
    }

    if let Some(TickCommand::Template { action }) = cli.command {
        return run_template_command(action).await;
    }

    if let Some(TickCommand::Issue { action }) = cli.command {
        return cli::issue::run(action).await;
    }

    if let Some(TickCommand::Search(args)) = cli.command {
        return cli::search::run(args)
            .await
            .map_err(|e| -> Box<dyn std::error::Error> { e.into() });
    }

    if let Some(TickCommand::Bulk { action }) = cli.command {
        return cli::bulk_cmd::run(action)
            .await
            .map_err(|e| -> Box<dyn std::error::Error> { e.into() });
    }

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

    if cli.check {
        std::process::exit(config_check::run_check(&config));
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
    execute!(
        stdout,
        EnterAlternateScreen,
        EnableMouseCapture,
        PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES),
    )?;
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

    let jira = std::sync::Arc::new(
        api::JiraClient::from_config(&config, cli.debug)
            .await
            .map_err(|e| format!("Auth error: {e}"))?,
    );
    let mut app = App::new(config, theme, jira, cli.debug);
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
                    should_quit = input::handle_key(&mut app, key).await;
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
        PopKeyboardEnhancementFlags,
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

async fn run_template_command(action: TemplateCommand) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        TemplateCommand::Export(args) => run_template_export(args).await,
    }
}

async fn run_template_export(args: TemplateExportArgs) -> Result<(), Box<dyn std::error::Error>> {
    if args.keys.is_empty() {
        return Err(
            "Provide at least one issue key (e.g. tick template export my-site HIN-1)".into(),
        );
    }

    let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
    let jira = api::JiraClient::from_config(&config, false)
        .await
        .map_err(|e| format!("Auth error: {e}"))?;

    let content =
        template_export::export_issues_to_toml(&config, &args.site, &args.keys, &jira).await?;

    if let Some(path) = args.output {
        template_export::write_templates_file(&path, &content, args.append)?;
        println!("Wrote {}", path.display());
    } else {
        print!("{content}");
    }
    Ok(())
}

async fn run_auth_command(action: AuthCommand) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        AuthCommand::Login => {
            let config = Config::load().map_err(|e| format!("Config error: {e}"))?;
            oauth::login(&config.oauth).await?;
        }
        AuthCommand::Status => {
            auth_status::print_status().await?;
        }
        AuthCommand::Logout => {
            oauth::delete_tokens()?;
            println!("OAuth tokens removed.");
        }
    }
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
