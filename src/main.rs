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
