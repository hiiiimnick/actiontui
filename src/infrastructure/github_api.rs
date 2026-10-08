use crate::Config;
use crate::domain::models::Logs;
use crate::domain::models::{Repository, Run, Workflow, WorkflowInput};
use crate::domain::repositories::WorkflowRepository;
use crate::domain::{Job, Step};
use crate::infrastructure::map_optional_time;
use crate::infrastructure::workflow_definition::parse_dispatch_inputs;
use chrono::{DateTime, Utc};
use color_eyre::Result;
use color_eyre::eyre::Ok;
use reqwest::blocking::Client;
use reqwest::header::{ACCEPT, AUTHORIZATION, USER_AGENT};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Default, Debug)]
pub struct HttpWorkflowRepository {
    cfg: Config,
    client: Client,
}

impl HttpWorkflowRepository {
    pub fn new(cfg: Config) -> Self {
        Self {
            cfg,
            client: Client::new(),
        }
    }

    fn get_request(&self, url: String) -> reqwest::blocking::RequestBuilder {
        self.client
            .get(url)
            .header(USER_AGENT, "actiontui")
            .header(AUTHORIZATION, format!("Bearer {}", self.cfg.pat))
            .header(ACCEPT, "application/vnd.github+json")
    }
}

#[derive(Deserialize)]
struct GithubWorkflowResponse {
    workflows: Vec<GithubWorkflow>,
}

#[derive(Deserialize)]
struct GithubWorkflow {
    id: u64,
    name: String,
    path: String,
    state: String,
}

#[derive(Deserialize)]
struct GithubRunResponse {
    workflow_runs: Vec<GithubWorkflowRun>,
}

#[derive(Deserialize)]
struct GithubWorkflowRun {
    id: u64,
    name: Option<String>,
    status: String,
    conclusion: Option<String>,
    workflow_id: u64,
    html_url: String,
    created_at: Option<DateTime<Utc>>,
    display_title: String,
    head_branch: String,
}

#[derive(Deserialize)]
struct GithubWorkflowRunJobStep {
    name: String,
    status: String,
    conclusion: Option<String>,
    number: u64,
    started_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
}
#[derive(Deserialize)]
struct GithubWorkflowRunJob {
    id: u64,
    status: String,
    conclusion: Option<String>,
    started_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
    name: String,
    steps: Vec<GithubWorkflowRunJobStep>,
}

#[derive(Deserialize)]
struct GithubJobResponse {
    jobs: Vec<GithubWorkflowRunJob>,
}

impl WorkflowRepository for HttpWorkflowRepository {
    fn get_workflows(&self, repo: &Repository) -> Result<Vec<Workflow>> {
        let url = format!(
            "https://api.{}/repos/{}/{}/actions/workflows",
            self.cfg.url, repo.owner, repo.repo
        );

        let response: GithubWorkflowResponse = self.get_request(url).send()?.json()?;

        Ok(response
            .workflows
            .into_iter()
            .map(|w| Workflow {
                id: w.id,
                name: w.name,
                path: w.path,
                state: w.state,
            })
            .collect())
    }

    fn get_runs(&self, repo: &Repository, workflow_id: u64) -> Result<Vec<Run>> {
        let url = format!(
            "https://api.{}/repos/{}/{}/actions/workflows/{}/runs",
            self.cfg.url, repo.owner, repo.repo, workflow_id
        );

        let response: GithubRunResponse = self.get_request(url).send()?.json()?;

        Ok(response
            .workflow_runs
            .into_iter()
            .map(|run| Run {
                id: run.id,
                name: run.name.unwrap_or_default(),
                status: run.status,
                conclusion: run.conclusion,
                workflow_id: run.workflow_id,
                html_url: run.html_url,
                created_at: map_optional_time(run.created_at),
                display_title: run.display_title,
                head_branch: run.head_branch,
            })
            .collect())
    }

    fn get_jobs(&self, repo: &Repository, run_id: u64) -> Result<Vec<Job>> {
        let url = format!(
            "https://api.{}/repos/{}/{}/actions/runs/{}/jobs",
            self.cfg.url, repo.owner, repo.repo, run_id
        );

        let response: GithubJobResponse = self.get_request(url).send()?.json()?;

        Ok(response
            .jobs
            .into_iter()
            .map(|job| Job {
                id: job.id,
                name: job.name,
                status: job.status,
                conclusion: job.conclusion,
                started_at: job.started_at.map(DateTime::from),
                completed_at: job.completed_at.map(DateTime::from),
                steps: job
                    .steps
                    .into_iter()
                    .map(|step| Step {
                        name: step.name,
                        status: step.status,
                        conclusion: step.conclusion,
                        number: step.number,
                        started_at: step.started_at,
                        completed_at: step.completed_at,
                    })
                    .collect(),
            })
            .collect())
    }

    fn get_job_by_id(&self, repo: &Repository, job_id: u64) -> Result<Job> {
        let url = format!(
            "https://api.{}/repos/{}/{}/actions/jobs/{}",
            self.cfg.url, repo.owner, repo.repo, job_id
        );

        let response: GithubWorkflowRunJob = self.get_request(url).send()?.json()?;
        Ok(Job {
            id: response.id,
            name: response.name,
            status: response.status,
            conclusion: response.conclusion,
            started_at: response.started_at.map(DateTime::from),
            completed_at: response.completed_at.map(DateTime::from),
            steps: response
                .steps
                .into_iter()
                .map(|step| Step {
                    name: step.name,
                    status: step.status,
                    conclusion: step.conclusion,
                    number: step.number,
                    started_at: step.started_at,
                    completed_at: step.completed_at,
                })
                .collect(),
        })
    }

    fn get_logs(&self, repo: &Repository, job_id: u64) -> Result<Logs> {
        let url = format!(
            "https://api.{}/repos/{}/{}/actions/jobs/{}/logs",
            self.cfg.url, repo.owner, repo.repo, job_id
        );
        let response = self.get_request(url).send()?.error_for_status()?;
        // streamed to disk, the log is never held in memory
        Ok(Logs::from_reader(response)?)
    }

    fn get_workflow_inputs(
        &self,
        repo: &Repository,
        workflow: &Workflow,
        reference: &str,
    ) -> Result<Vec<WorkflowInput>> {
        let url = format!(
            "https://api.{}/repos/{}/{}/contents/{}",
            self.cfg.url,
            repo.owner,
            repo.repo,
            workflow.path.trim_start_matches('/')
        );

        // the raw media type returns the file itself instead of base64 JSON
        let yaml = self
            .get_request(url)
            .query(&[("ref", reference)])
            .header(ACCEPT, "application/vnd.github.raw+json")
            .send()?
            .error_for_status()?
            .text()?;
        parse_dispatch_inputs(&yaml)
    }

    fn trigger_workflow(
        &self,
        repo: &Repository,
        workflow_id: u64,
        reference: &str,
        inputs: &HashMap<String, String>,
    ) -> Result<()> {
        let url = format!(
            "https://api.{}/repos/{}/{}/actions/workflows/{}/dispatches",
            self.cfg.url, repo.owner, repo.repo, workflow_id
        );

        let mut body = serde_json::json!({
            "ref": reference
        });
        if !inputs.is_empty() {
            body["inputs"] = serde_json::json!(inputs);
        }

        let res = self
            .client
            .post(url)
            .header(USER_AGENT, "actiontui")
            .header(AUTHORIZATION, format!("Bearer {}", self.cfg.pat))
            .header(ACCEPT, "application/vnd.github+json")
            .json(&body)
            .send()?;

        if !res.status().is_success() {
            return Err(color_eyre::eyre::eyre!(
                "Failed to trigger workflow: {}",
                res.text()?
            ));
        }

        Ok(())
    }
}
