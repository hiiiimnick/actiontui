use std::collections::HashMap;
use std::fs::File;

use color_eyre::eyre::Error;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::Terminal;
use ratatui::backend::Backend;
use ratatui::widgets::ListState;
use tui_widget_list;

use crate::Config;
use crate::domain::{Job, Repository, Run, Step, Workflow, WorkflowRepository};
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
    Logs,
}

#[derive(Debug)]
pub struct App {
    pub cfg: Config,
    pub repo: Repository,
    pub current_focus: CurrentFocus,
    pub mode: Mode,
    pub workflowrepo: Box<dyn WorkflowRepository>,

    pub workflows: Vec<Workflow>,
    pub workflow_state: tui_widget_list::ListState,

    pub selected_workflow_id: Option<u64>,
    pub runs: Vec<Run>,
    pub run_state: ListState,

    pub selected_run_id: Option<u64>,
    pub jobs: Vec<Job>,
    pub job_state: ListState,

    pub selected_job: Option<Job>,
    pub logs: Option<File>,
    pub log_index: HashMap<String, (u64, u64)>,

    pub step_state: ListState,
    pub selected_step: Option<Step>,
}

impl App {
    pub fn new(cfg: Config, repo: Repository) -> Result<App, Error> {
        let workflow_repo = Box::new(HttpWorkflowRepository::new(cfg.clone()));
        let workflows = workflow_repo.get_workflows(&repo)?;
        let mut workflow_state = tui_widget_list::ListState::default();
        if !workflows.is_empty() {
            workflow_state.select(Some(0));
        }

        Ok(App {
            cfg,
            repo,
            workflowrepo: workflow_repo,
            workflows,
            workflow_state,
            current_focus: CurrentFocus::Workflows,
            mode: Mode::Navigation,
            selected_workflow_id: None,
            run_state: ListState::default(),
            runs: Vec::new(),
            selected_run_id: None,
            jobs: Vec::new(),
            selected_job: None,
            job_state: ListState::default(),
            logs: None,
            log_index: HashMap::default(),
            step_state: ListState::default(),
            selected_step: None,
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
            CurrentFocus::Workflows => match key.code {
                KeyCode::Char('k') | KeyCode::Up => {
                    self.workflow_state.previous();
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    self.workflow_state.next();
                }
                KeyCode::Char('K') | KeyCode::Home => {
                    self.workflow_state.select(Some(0));
                }
                KeyCode::Char('J') | KeyCode::End => {
                    self.workflow_state.select(Some(self.workflows.len() - 1));
                }
                KeyCode::Char('r') => {
                    self.workflows =
                        HttpWorkflowRepository::new(self.cfg.clone()).get_workflows(&self.repo)?;
                }
                KeyCode::Enter => {
                    if let Some(index) = self.workflow_state.selected
                        && let Some(workflow) = self.workflows.get(index)
                    {
                        self.selected_workflow_id = Some(workflow.id);
                        self.runs = self.workflowrepo.get_runs(&self.repo, workflow.id)?;
                        self.current_focus = CurrentFocus::Runs;
                        self.workflow_state = tui_widget_list::ListState::default();
                    }
                }
                _ => {}
            },
            CurrentFocus::Runs => {
                navigate_list(key, &mut self.run_state);
                match key.code {
                    KeyCode::Char('r') => {
                        if let Some(workflow_id) = &self.selected_workflow_id {
                            self.runs = self.workflowrepo.get_runs(&self.repo, *workflow_id)?;
                        }
                    }
                    KeyCode::Enter => {
                        if let Some(index) = self.run_state.selected()
                            && let Some(run) = self.runs.get(index)
                        {
                            self.selected_run_id = Some(run.id);
                            self.jobs = self.workflowrepo.get_jobs(&self.repo, run.id)?;
                            self.current_focus = CurrentFocus::Jobs;
                            self.run_state = ListState::default();
                        }
                    }
                    _ => {}
                }
            }
            CurrentFocus::Jobs => {
                navigate_list(key, &mut self.job_state);
                match key.code {
                    KeyCode::Char('r') => {
                        if let Some(run_id) = &self.selected_run_id {
                            self.jobs = self.workflowrepo.get_jobs(&self.repo, *run_id)?;
                        }
                    }
                    KeyCode::Enter => {
                        if let Some(index) = self.job_state.selected()
                            && let Some(job) = self.jobs.get(index)
                        {
                            self.selected_job = Some(job.clone());
                            self.current_focus = CurrentFocus::Logs;
                            self.job_state = ListState::default();
                            let logs = self.workflowrepo.get_logs(&self.repo, job.id).unwrap();
                            self.logs = Some(logs.save_to_file().unwrap());
                            self.log_index = logs.create_step_index(job.steps.clone()).unwrap();
                        }
                    }
                    _ => {}
                }
            }
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
