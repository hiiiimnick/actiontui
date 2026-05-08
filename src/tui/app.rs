use std::io::{self};

use crossterm::event::{self, Event, KeyCode};
use ratatui::Terminal;
use ratatui::backend::Backend;

use crate::Config;
use crate::domain::models::Workflow;
use crate::tui::ui;

#[derive(Debug)]
pub enum Mode {
    Navigation,
    Input,
}

#[derive(Debug, Eq, PartialEq)]
pub enum CurrentFocus {
    Workflow,
    WorkflowRun,
    Steps,
}

#[derive(Debug, Default)]
pub struct App {
    pub cfg: Config,
    pub current_focus: CurrentFocus,

    pub workflows: Vec<Workflow>,
}

impl App {
    pub fn new(cfg: Config, workflows: Vec<Workflow>) -> App {
        App {
            cfg,
            workflows,
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
        Self::Workflow
    }
}
