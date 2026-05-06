use color_eyre::eyre::{Result, eyre};
use config::Config;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use reqwest::Error;
use std::{
    io,
    process::{Command, exit},
};
use tui::app::App;

use crate::{domain::Repository, rest::workflow_service};

mod config;
mod domain;
mod rest;
mod tui;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let cfg: Config = confy::load("actiontui", "config")?;
    let repo = Repository::parse_current()?;
    let workflows =
        workflow_service::get_workflows(&cfg, repo).expect("Api Request to Github failed");

    let mut app = App::new(cfg, workflows);

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

impl Repository {
    pub fn parse_current() -> Result<Self> {
        let output = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .arg("/C")
                .arg("git config --get remote.origin.url")
                .output()?
        } else {
            Command::new("sh")
                .arg("-c")
                .arg("git config --get remote.origin.url")
                .output()?
        };

        if !output.status.success() {
            return Err(eyre!(
                "Failed to get remote origin URL. Are you in a git repository?"
            ));
        }

        let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Self::parse_url(&url)
    }

    pub fn parse_url(url: &str) -> Result<Self> {
        let path = if let Some(ssh_part) = url.strip_prefix("git@") {
            ssh_part
                .split_once(':')
                .map(|x| x.1)
                .ok_or_else(|| eyre!("Invalid SSH URL: {}", url))?
        } else {
            url.trim_start_matches("https://")
                .trim_start_matches("http://")
                .split_once('/')
                .map(|x| x.1)
                .ok_or_else(|| eyre!("Invalid HTTP URL: {}", url))?
        };

        let path = path.trim_end_matches(".git");
        let parts: Vec<&str> = path.split('/').collect();

        if parts.len() < 2 {
            return Err(eyre!(
                "Could not determine owner and repo from URL: {}",
                url
            ));
        }

        Ok(Repository {
            owner: parts[parts.len() - 2].to_string(),
            repo: parts[parts.len() - 1].to_string(),
        })
    }
}
