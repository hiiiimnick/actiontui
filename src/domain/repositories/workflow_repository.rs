use crate::domain::models::{Repository, Run, Workflow};
use color_eyre::Result;

pub trait WorkflowRepository {
    fn get_workflows(&self, repo: &Repository) -> Result<Vec<Workflow>>;
    fn get_runs(&self, repo: &Repository, workflow_id: u64) -> Result<Vec<Run>>;
    fn trigger_workflow(&self, repo: &Repository, workflow_id: u64, reference: &str) -> Result<()>;
}
