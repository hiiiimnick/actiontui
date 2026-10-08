use color_eyre::eyre::Error;
use std::collections::HashMap;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::Terminal;
use ratatui::backend::Backend;
use ratatui::widgets::{List, ListState};
use tui_widget_list;

use crate::Config;
use crate::domain::{
    Job, Logs, Repository, Run, Step, StepLogIndex, StepLogLocator, Workflow, WorkflowInput,
    WorkflowRepository,
};
use crate::infrastructure::HttpWorkflowRepository;
use crate::tui::run_form::{FormAction, RunForm};
use crate::tui::ui;

#[derive(Debug, Eq, PartialEq)]
pub enum Mode {
    Navigation,
    Input,
}

/// Feedback for the user, shown in the bottom line until the next key press.
#[derive(Debug)]
pub struct StatusMessage {
    pub text: String,
    pub is_error: bool,
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
    pub repo: Repository,
    pub current_focus: CurrentFocus,
    pub mode: Mode,
    pub run_form: Option<RunForm>,
    pub message: Option<StatusMessage>,
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
    pub logs: Option<Logs>,
    pub log_index: StepLogIndex,

    pub step_state: ListState,
    pub selected_step: Option<Step>,

    pub log_lines: Vec<String>,
    pub logs_offset: u64,
}

impl App {
    pub fn new(cfg: Config, repo: Repository) -> Result<App, Error> {
        let workflow_repo = Box::new(HttpWorkflowRepository::new(cfg));
        let workflows = workflow_repo.get_workflows(&repo)?;
        let mut workflow_state = tui_widget_list::ListState::default();
        if !workflows.is_empty() {
            workflow_state.select(Some(0));
        }

        Ok(App {
            repo,
            workflowrepo: workflow_repo,
            workflows,
            workflow_state,
            current_focus: CurrentFocus::Workflows,
            mode: Mode::Navigation,
            run_form: None,
            message: None,
            selected_workflow_id: None,
            run_state: ListState::default(),
            runs: Vec::new(),
            selected_run_id: None,
            jobs: Vec::new(),
            selected_job: None,
            job_state: ListState::default(),
            logs: None,
            log_index: StepLogIndex::default(),
            step_state: ListState::default(),
            selected_step: None,
            log_lines: Vec::new(),
            logs_offset: 0,
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
        self.message = None;
        match self.mode {
            Mode::Input => self.handle_key_input_form(key),
            Mode::Navigation => return self.handle_key_input_navigation(key),
        }
        Ok(false)
    }

    /// Opens the form to start a new run of `workflow_id`, with the inputs
    /// the workflow declares on the current branch.
    fn open_run_form(&mut self, workflow_id: u64) {
        let Some(workflow) = self.workflows.iter().find(|w| w.id == workflow_id) else {
            return;
        };
        let branch = Repository::current_branch().unwrap_or_else(|_| "main".into());
        let (inputs, notice) = self.load_inputs(workflow, &branch);
        self.run_form = Some(RunForm::new(
            workflow.id,
            workflow.name.clone(),
            branch,
            inputs,
            notice,
        ));
        self.mode = Mode::Input;
    }

    /// The inputs of `workflow` on `branch`. If they cannot be read the run
    /// can still be started without inputs, so the error is only a notice.
    fn load_inputs(
        &self,
        workflow: &Workflow,
        branch: &str,
    ) -> (Vec<WorkflowInput>, Option<String>) {
        match self
            .workflowrepo
            .get_workflow_inputs(&self.repo, workflow, branch)
        {
            Ok(inputs) => (inputs, None),
            Err(e) => (Vec::new(), Some(format!("Could not read inputs: {e}"))),
        }
    }

    fn handle_key_input_form(&mut self, key: KeyEvent) {
        let Some(form) = self.run_form.as_mut() else {
            self.mode = Mode::Navigation;
            return;
        };
        match form.handle_key(key) {
            FormAction::None => {}
            FormAction::Cancel => {
                self.run_form = None;
                self.mode = Mode::Navigation;
            }
            FormAction::Submit => {
                let (branch, inputs) = form.values();
                let workflow_id = form.workflow_id;
                self.run_form = None;
                self.mode = Mode::Navigation;
                self.trigger_run(workflow_id, &branch, &inputs);
            }
            FormAction::BranchChanged => {
                let branch = form.branch().to_string();
                let workflow = self.workflows.iter().find(|w| w.id == form.workflow_id);
                if let Some(workflow) = workflow {
                    let (inputs, notice) = self.load_inputs(workflow, &branch);
                    if let Some(form) = self.run_form.as_mut() {
                        form.set_inputs(inputs, notice);
                    }
                }
            }
        }
    }

    fn trigger_run(&mut self, workflow_id: u64, branch: &str, inputs: &HashMap<String, String>) {
        let name = self
            .workflows
            .iter()
            .find(|w| w.id == workflow_id)
            .map_or("workflow", |w| w.name.as_str());
        self.message = Some(
            match self
                .workflowrepo
                .trigger_workflow(&self.repo, workflow_id, branch, inputs)
            {
                Ok(()) => StatusMessage {
                    text: format!("Started '{name}' on {branch}, refresh runs with r"),
                    is_error: false,
                },
                Err(e) => StatusMessage {
                    text: format!("Could not start '{name}': {e}"),
                    is_error: true,
                },
            },
        );
    }

    fn handle_key_input_navigation(&mut self, key: KeyEvent) -> Result<bool, Error> {
        match key.code {
            KeyCode::Char('q') => {
                return Ok(true);
            }
            KeyCode::Char('1') => {
                self.current_focus = CurrentFocus::Workflows;
                self.set_default_states();
            }
            KeyCode::Char('2') => {
                self.current_focus = CurrentFocus::Runs;
                self.set_default_states();
            }
            KeyCode::Char('3') => {
                self.current_focus = CurrentFocus::Jobs;
                self.set_default_states();
            }
            KeyCode::Char('4') => {
                self.current_focus = CurrentFocus::Steps;
                self.set_default_states();
            }
            KeyCode::Char('5') => {
                self.current_focus = CurrentFocus::Logs;
                self.set_default_states();
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
                    self.workflows = self.workflowrepo.get_workflows(&self.repo)?;
                }
                KeyCode::Char('n') => {
                    if let Some(index) = self.workflow_state.selected
                        && let Some(workflow) = self.workflows.get(index)
                    {
                        self.open_run_form(workflow.id);
                    }
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
                    KeyCode::Char('n') => {
                        if let Some(workflow_id) = self.selected_workflow_id {
                            self.open_run_form(workflow_id);
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
                            self.current_focus = CurrentFocus::Steps;
                            self.job_state = ListState::default();
                            let logs = self.workflowrepo.get_logs(&self.repo, job.id)?;
                            self.log_index = StepLogLocator::locate(&logs, &job.steps)?;
                            self.logs = Some(logs);
                        }
                    }
                    _ => {}
                }
            }
            CurrentFocus::Steps => {
                navigate_list(key, &mut self.step_state);
                match key.code {
                    KeyCode::Char('r') => {
                        if let Some(selected_job) = &self.selected_job {
                            self.selected_job = Option::from(
                                self.workflowrepo
                                    .get_job_by_id(&self.repo, selected_job.id)?,
                            );
                        }
                    }
                    KeyCode::Enter => {
                        if let Some(index) = self.step_state.selected()
                            && let Some(job) = &self.selected_job
                            && let Some(step) = job.steps.get(index)
                        {
                            self.selected_step = Some(step.clone());
                            self.current_focus = CurrentFocus::Logs;
                            self.logs_offset = 0;
                            self.load_step_logs()?;
                        }
                    }
                    _ => {}
                }
            }
            CurrentFocus::Logs => {
                let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
                let step = if ctrl { 10 } else { 1 };
                match key.code {
                    KeyCode::Char('j') | KeyCode::Down => {
                        self.logs_offset = self.logs_offset.saturating_add(step);
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        self.logs_offset = self.logs_offset.saturating_sub(step);
                    }
                    // clamped to the last page while rendering
                    KeyCode::Char('G') | KeyCode::Char('J') | KeyCode::End => {
                        self.logs_offset = u64::MAX
                    }
                    KeyCode::Char('g') | KeyCode::Char('K') | KeyCode::Home => self.logs_offset = 0,
                    _ => {}
                }
            }
        }
        Ok(false)
    }

    fn load_step_logs(&mut self) -> Result<(), Error> {
        self.log_lines = match (&self.logs, &self.selected_step) {
            (Some(logs), Some(step)) => logs.lines_in(self.log_index.range_of(step))?,
            _ => Vec::new(),
        };
        Ok(())
    }

    fn set_default_states(&mut self) {
        self.job_state = ListState::default();
        self.run_state = ListState::default();
        self.step_state = ListState::default();
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
