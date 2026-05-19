use crate::domain::models::{Job, Logs, Repository, Run, Workflow};
use color_eyre::Result;

pub trait WorkflowRepository {
    fn get_workflows(&self, repo: &Repository) -> Result<Vec<Workflow>>;
    fn get_runs(&self, repo: &Repository, workflow_id: u64) -> Result<Vec<Run>>;
    fn get_jobs(&self, repo: &Repository, run_id: u64) -> Result<Vec<Job>>;
    fn get_logs(&self, repo: &Repository, job_id: u64) -> Result<Logs>;
    fn trigger_workflow(&self, repo: &Repository, workflow_id: u64, reference: &str) -> Result<()>;
}
