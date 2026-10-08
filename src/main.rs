use clap::Parser;
use color_eyre::eyre::Result;
use color_eyre::eyre::WrapErr;
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

use crate::domain::models::Repository;

mod config;
mod domain;
mod infrastructure;
mod tui;

/// A TUI to run and monitor GitHub Actions.
#[derive(Parser)]
#[command(version)]
struct Cli {
    /// Use this profile of the config instead of detecting it from the git remote.
    #[arg(short, long)]
    profile: Option<String>,
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();

    let cfg: Config = confy::load("actiontui", "config")?;
    let repo = Repository::parse_current()?;

    let account = cfg
        .select_profile(cli.profile.as_deref(), &repo.host, &repo.owner)
        .and_then(|profile| profile.account())
        .wrap_err_with(|| {
            let path = confy::get_configuration_file_path("actiontui", "config");
            format!(
                "Check the configuration: {}",
                path.map_or_else(|_| "actiontui config".into(), |p| p.display().to_string())
            )
        })?;

    let mut app = App::new(account, repo)?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    app.run(&mut terminal)?;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
