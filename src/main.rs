use clap::{Parser, Subcommand};
use color_eyre::eyre::{Result, WrapErr, eyre};
use config::Config;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::io;
use tui::app::App;

use crate::domain::RepositoryCatalog;
use crate::domain::models::Repository;
use crate::infrastructure::HttpWorkflowRepository;
use crate::tui::repo_picker::RepoEntry;

mod config;
mod domain;
mod infrastructure;
mod tui;

/// A TUI to run and monitor GitHub Actions.
#[derive(Parser)]
#[command(version)]
struct Cli {
    /// Use this profile of the config instead of detecting it from the git remote.
    #[arg(short, long, global = true)]
    profile: Option<String>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Start with a searchable list of all repositories of all profiles
    /// (or of the one given with --profile) instead of the current repository.
    Global,
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();

    let cfg: Config = confy::load("actiontui", "config")?;
    let config_hint = || {
        let path = confy::get_configuration_file_path("actiontui", "config");
        format!(
            "Check the configuration: {}",
            path.map_or_else(|_| "actiontui config".into(), |p| p.display().to_string())
        )
    };

    let app = match cli.command {
        Some(Command::Global) => {
            global_app(&cfg, cli.profile.as_deref()).wrap_err_with(config_hint)?
        }
        None => {
            let repo = Repository::parse_current()?;
            let account = cfg
                .select_profile(cli.profile.as_deref(), &repo.host, &repo.owner)
                .and_then(|profile| profile.account())
                .wrap_err_with(config_hint)?;
            App::new(account, repo)?
        }
    };

    run(app)
}

/// The app showing all repositories of the profiles in the config. A profile
/// that cannot be loaded does not stop the others, it is reported in the list.
fn global_app(cfg: &Config, requested: Option<&str>) -> Result<App> {
    let mut profiles = cfg.all_profiles();
    if let Some(name) = requested {
        if !profiles.iter().any(|p| p.name == name) {
            let available: Vec<&str> = profiles.iter().map(|p| p.name.as_str()).collect();
            return Err(eyre!(
                "Unknown profile '{name}'. Available: {}",
                available.join(", ")
            ));
        }
        profiles.retain(|p| p.name == name);
    }

    let mut accounts = Vec::new();
    let mut entries = Vec::new();
    let mut notices = Vec::new();
    for profile in profiles {
        eprintln!("Loading repositories of '{}' ...", profile.name);
        let loaded = profile.account().and_then(|account| {
            let summaries = HttpWorkflowRepository::new(account.clone()).list_repositories()?;
            Ok((account, summaries))
        });
        match loaded {
            Ok((account, summaries)) => {
                entries.extend(summaries.into_iter().map(|summary| RepoEntry {
                    profile: account.profile.clone(),
                    summary,
                }));
                accounts.push(account);
            }
            Err(error) => notices.push(format!("{}: {error}", profile.name)),
        }
    }

    if accounts.is_empty() {
        return Err(eyre!(
            "No repositories could be loaded:\n{}",
            notices.join("\n")
        ));
    }
    App::new_global(accounts, entries, notices)
}

fn run(mut app: App) -> Result<()> {
    // a panic or an error must not leave the terminal in raw mode
    let panic_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = restore_terminal();
        panic_hook(info);
    }));

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let result = app.run(&mut terminal);

    restore_terminal()?;
    terminal.show_cursor()?;
    result?;

    Ok(())
}

fn restore_terminal() -> io::Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)
}
