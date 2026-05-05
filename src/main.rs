use color_eyre::eyre::Error;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use serde_derive::{Deserialize, Serialize};
use std::{io, process::Command};
use tui::app::App;

use crate::{domain::Repository, rest::workflow_service};

mod domain;
mod rest;
mod tui;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    url: String,
    pat: String,
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let cfg: Config = confy::load("actiontui", "config")?;
    dbg!(&cfg);
    let repo = Repository {
        owner: "hiiiimnick".to_string(),
        repo: "actiontui".to_string(),
    };
    let workflows = workflow_service::get_workflows(&cfg, repo);

    match workflows {
        Ok(list) => {
            println!("{:#?}", list);
        }
        Err(error) => {
            println!("{:?}", error);
        }
    }

    let mut app = App::new(cfg);

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

fn repo_parser() -> Repository {
    let output = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .arg("/C")
            .arg("git config --get remote.origin.url")
            .output()
            .expect("no remote url found, not in a git repo?")
    } else {
        Command::new("sh")
            .arg("-c")
            .arg("git config --get remote.origin.url")
            .output()
            .expect("no remote url found, not in a git repo?")
    };

    let substrings = String::from_utf8_lossy(&output.stdout)
        .to_string()
        .split("/").collect();

    let owner = substrings[substrings.Rc::new
}

impl ::std::default::Default for Config {
    fn default() -> Self {
        Self {
            url: "https://www.github.com".into(),
            pat: "".into(),
        }
    }
}
