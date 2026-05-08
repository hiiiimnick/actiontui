use crate::domain::models::{Repository, Workflow, WorkflowRun};
use color_eyre::Result;

pub trait WorkflowRepository {
    fn get_workflows(&self, repo: &Repository) -> Result<Vec<Workflow>>;
    fn get_runs(&self, repo: &Repository, workflow_id: u64) -> Result<Vec<WorkflowRun>>;
    fn trigger_workflow(&self, repo: &Repository, workflow_id: u64, reference: &str) -> Result<()>;
}
