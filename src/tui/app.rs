use color_eyre::eyre::Error;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::Terminal;
use ratatui::backend::Backend;
use ratatui::widgets::ListState;

use crate::Config;
use crate::domain::models::run;
use crate::domain::{Job, Repository, Run, Workflow, WorkflowRepository};
use crate::infrastructure::HttpWorkflowRepository;
use crate::tui::ui;

#[derive(Debug, Eq, PartialEq)]
pub enum Mode {
    Navigation,
    Input,
}

#[derive(Debug, Eq, PartialEq)]
pub enum CurrentFocus {
    Workflows,
    Runs,
    Jobs,
    Steps,
    Logs,
}

#[derive(Debug)]
pub struct App {
    pub cfg: Config,
    pub repo: Repository,
    pub current_focus: CurrentFocus,
    pub mode: Mode,

    pub workflows: Vec<Workflow>,
    pub workflow_state: ListState,

    pub selected_workflow: Option<Workflow>,
    pub runs: Vec<Run>,
    pub run_state: ListState,

    pub selected_run: Option<Run>,
    pub jobs: Vec<Job>,

    pub selected_job: Option<Job>,
    pub job_state: ListState,
}

impl App {
    pub fn new(cfg: Config, repo: Repository) -> Result<App, Error> {
        let workflow_repo = HttpWorkflowRepository::new(cfg.clone());
        let workflows = workflow_repo.get_workflows(&repo)?;
        let mut workflow_state = ListState::default();
        if !workflows.is_empty() {
            workflow_state.select(Some(0));
        }

        Ok(App {
            cfg,
            repo,
            workflows,
            workflow_state,
            current_focus: CurrentFocus::Workflows,
            mode: Mode::Navigation,
            selected_workflow: None,
            run_state: ListState::default(),
            runs: Vec::new(),
            selected_run: None,
            jobs: Vec::new(),
            selected_job: None,
            job_state: ListState::default(),
        })
    }

    pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<bool, Error>
    where
        Error: From<B::Error>,
    {
        loop {
            terminal.draw(|f| ui::ui(self, f))?;

            if let Event::Key(key) = event::read()?
                && self.handle_key_input(key)?
            {
                return Ok(true);
            }
        }
    }

    fn handle_key_input(&mut self, key: KeyEvent) -> Result<bool, Error> {
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return Ok(true);
        }
        match self.mode {
            Mode::Input => {}
            Mode::Navigation => return self.handle_key_input_navigation(key),
        }
        Ok(false)
    }

    fn handle_key_input_navigation(&mut self, key: KeyEvent) -> Result<bool, Error> {
        match key.code {
            KeyCode::Char('q') => {
                return Ok(true);
            }
            KeyCode::Char('1') => {
                self.current_focus = CurrentFocus::Workflows;
                self.run_state = ListState::default();
            }
            KeyCode::Char('2') => {
                self.current_focus = CurrentFocus::Runs;
            }
            _ => {}
        }

        match self.current_focus {
            CurrentFocus::Workflows => {
                navigate_list(key, &mut self.workflow_state);
                match key.code {
                    KeyCode::Char('r') => {
                        self.workflows = HttpWorkflowRepository::new(self.cfg.clone())
                            .get_workflows(&self.repo)?;
                    }
                    KeyCode::Enter => {
                        if let Some(index) = self.workflow_state.selected()
                            && let Some(workflow) = self.workflows.get(index)
                        {
                            self.selected_workflow = Some(workflow.clone());
                            self.runs = HttpWorkflowRepository::new(self.cfg.clone())
                                .get_runs(&self.repo, workflow.id)?;
                            self.current_focus = CurrentFocus::Runs;
                            self.workflow_state = ListState::default();
                        }
                    }
                    _ => {}
                }
            }
            CurrentFocus::Runs => {
                navigate_list(key, &mut self.run_state);
                match key.code {
                    KeyCode::Char('r') => {
                        if let Some(workflow) = &self.selected_workflow {
                            self.runs = HttpWorkflowRepository::new(self.cfg.clone())
                                .get_runs(&self.repo, workflow.id)?;
                        }
                    }
                    KeyCode::Enter => {
                        if let Some(index) = self.run_state.selected()
                            && let Some(run) = self.runs.get(index)
                        {
                            self.selected_run = Some(run.clone());
                            self.jobs = HttpWorkflowRepository::new(self.cfg.clone())
                                .get_jobs(&self.repo, run.id)?;
                            self.current_focus = CurrentFocus::Jobs;
                            self.run_state = ListState::default();
                        }
                    }
                    _ => {}
                }
            }
            CurrentFocus::Jobs => {
                navigate_list(key, &mut self.job_state);
            }
            CurrentFocus::Steps => {}
            CurrentFocus::Logs => {}
        }
        Ok(false)
    }
}

fn navigate_list(key: KeyEvent, list_state: &mut ListState) {
    match key.code {
        KeyCode::Char('k') | KeyCode::Up => {
            list_state.select_previous();
        }
        KeyCode::Char('j') | KeyCode::Down => {
            list_state.select_next();
        }
        KeyCode::Char('K') | KeyCode::Home => {
            list_state.select_first();
        }
        KeyCode::Char('J') | KeyCode::End => {
            list_state.select_last();
        }
        _ => {}
    }
}
