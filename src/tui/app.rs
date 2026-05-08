use std::io;

use color_eyre::eyre::Error;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::Terminal;
use ratatui::backend::Backend;
use ratatui::widgets::ListState;

use crate::Config;
use crate::domain::models::Workflow;
use crate::tui::ui;

#[derive(Debug, Default, Eq, PartialEq)]
pub enum Mode {
    #[default]
    Navigation,
    Input,
}

#[derive(Debug, Eq, PartialEq, Default)]
pub enum CurrentFocus {
    #[default]
    Workflows,
    Runs,
    Steps,
}

#[derive(Debug, Default)]
pub struct App {
    pub cfg: Config,
    pub current_focus: CurrentFocus,
    pub mode: Mode,

    pub workflows: Vec<Workflow>,
    pub workflow_state: ListState,
}

impl App {
    pub fn new(cfg: Config, workflows: Vec<Workflow>) -> App {
        App {
            cfg,
            workflows,
            workflow_state: ListState::default(),
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
                match self.handle_key_input(key) {
                    Ok(quit) => {
                        if quit {
                            return Ok(true);
                        }
                    }
                    Err(error) => return Err(error),
                }
            }
        }
    }

    fn handle_key_input(&mut self, key: KeyEvent) -> io::Result<bool> {
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return Ok(true);
        }
        match self.mode {
            Mode::Input => {}
            Mode::Navigation => {
                if key.code == KeyCode::Char('q') {
                    return Ok(true);
                }

                match self.current_focus {
                    CurrentFocus::Workflows => {
                        Self::navigate_list(key, &mut self.workflow_state);
                    }
                    CurrentFocus::Runs => {}
                    CurrentFocus::Steps => {}
                }
            }
        }
        Ok(false)
    }

    fn navigate_list(key: KeyEvent, list_state: &mut ListState) {
        match key.code {
            KeyCode::Char('k') => {
                list_state.select_previous();
            }
            KeyCode::Char('j') => {
                list_state.select_next();
            }
            KeyCode::Char('K') => {
                list_state.select_first();
            }
            KeyCode::Char('J') => {
                list_state.select_last();
            }
            _ => {}
        }
    }
}
