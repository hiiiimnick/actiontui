use std::{collections::HashMap, fmt::Debug};

use crate::domain::models::{
    Job, Logs, PendingDeployment, RateLimit, Repository, ReviewState, Run, Workflow, WorkflowInput,
};
use color_eyre::Result;

pub trait WorkflowRepository: Debug {
    /// The API quota as reported by the most recent request, if there was one.
    fn rate_limit(&self) -> Option<RateLimit>;
    fn get_workflows(&self, repo: &Repository) -> Result<Vec<Workflow>>;
    fn get_runs(&self, repo: &Repository, workflow_id: u64) -> Result<Vec<Run>>;
    fn get_jobs(&self, repo: &Repository, run_id: u64) -> Result<Vec<Job>>;
    fn get_logs(&self, repo: &Repository, job_id: u64) -> Result<Logs>;
    fn get_job_by_id(&self, repo: &Repository, job_id: u64) -> Result<Job>;
    /// The inputs `workflow` asks for when started manually on `reference`.
    fn get_workflow_inputs(
        &self,
        repo: &Repository,
        workflow: &Workflow,
        reference: &str,
    ) -> Result<Vec<WorkflowInput>>;
    /// The environments `run_id` is waiting to deploy to until they are approved.
    fn get_pending_deployments(
        &self,
        repo: &Repository,
        run_id: u64,
    ) -> Result<Vec<PendingDeployment>>;
    /// Approves or rejects the deployments to `environment_ids` of `run_id`.
    fn review_deployments(
        &self,
        repo: &Repository,
        run_id: u64,
        environment_ids: &[u64],
        state: ReviewState,
        comment: &str,
    ) -> Result<()>;
    /// Runs every job of `run_id` again.
    fn rerun_run(&self, repo: &Repository, run_id: u64) -> Result<()>;
    /// Runs `job_id` (and the jobs depending on it) again.
    fn rerun_job(&self, repo: &Repository, job_id: u64) -> Result<()>;
    /// Runs the failed jobs of `run_id` (and the jobs depending on them) again.
    fn rerun_failed_jobs(&self, repo: &Repository, run_id: u64) -> Result<()>;
    fn trigger_workflow(
        &self,
        repo: &Repository,
        workflow_id: u64,
        reference: &str,
        inputs: &HashMap<String, String>,
    ) -> Result<()>;
}
