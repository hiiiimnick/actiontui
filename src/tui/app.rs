use std::io::{self};

use crossterm::event::{self, Event, KeyCode};
use ratatui::Terminal;
use ratatui::backend::Backend;

use crate::Config;
use crate::tui::ui;

#[derive(Debug)]
pub enum CurrentFocus {
    RepoSelector,
    WorkflowSelector,
    WorkflowRunSelector,
    StepSelector,
}

#[derive(Debug)]
pub enum CurrentScreen {
    RepoList,
    WorkflowManager,
}

#[derive(Debug, Default)]
pub struct App {
    pub cfg: Config,
    pub current_focus: CurrentFocus,
    pub current_screen: CurrentScreen,
}

impl App {
    pub fn new(cfg: Config) -> App {
        App {
            cfg: cfg,
            ..Default::default()
        }
    }

    pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> io::Result<bool>
    where
        io::Error: From<B::Error>,
    {
        loop {
            terminal.draw(|f| ui::ui(self, f))?;
            if let Event::Key(key) = event::read()? {
                dbg!(key.code);
                match key.code {
                    KeyCode::Char('q') => {
                        return Ok(true);
                    }
                    _ => {}
                }
            }
        }
    }
}

impl ::std::default::Default for CurrentFocus {
    fn default() -> Self {
        Self::RepoSelector
    }
}
impl ::std::default::Default for CurrentScreen {
    fn default() -> Self {
        Self::WorkflowManager
    }
}
