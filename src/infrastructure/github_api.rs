use crate::Config;
use crate::domain::models::{Repository, Workflow, WorkflowRun};
use crate::domain::repositories::WorkflowRepository;
use color_eyre::Result;
use reqwest::blocking::Client;
use reqwest::header::{ACCEPT, AUTHORIZATION, USER_AGENT};
use serde::Deserialize;

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
                state: w.state,
            })
            .collect())
    }

    fn get_runs(&self, repo: &Repository, workflow_id: u64) -> Result<Vec<WorkflowRun>> {
        let url = format!(
            "https://api.{}/repos/{}/{}/actions/workflows/{}/runs",
            self.cfg.url, repo.owner, repo.repo, workflow_id
        );

        let response: GithubRunResponse = self.get_request(url).send()?.json()?;

        Ok(response
            .workflow_runs
            .into_iter()
            .map(|r| WorkflowRun {
                id: r.id,
                name: r.name.unwrap_or_default(),
                status: r.status,
                conclusion: r.conclusion,
                workflow_id: r.workflow_id,
                html_url: r.html_url,
            })
            .collect())
    }

    fn trigger_workflow(&self, repo: &Repository, workflow_id: u64, reference: &str) -> Result<()> {
        let url = format!(
            "https://api.{}/repos/{}/{}/actions/workflows/{}/dispatches",
            self.cfg.url, repo.owner, repo.repo, workflow_id
        );

        let body = serde_json::json!({
            "ref": reference
        });

        let res = self.client
            .post(url)
            .header(USER_AGENT, "actiontui")
            .header(AUTHORIZATION, format!("Bearer {}", self.cfg.pat))
            .header(ACCEPT, "application/vnd.github+json")
            .json(&body)
            .send()?;

        if !res.status().is_success() {
            return Err(color_eyre::eyre::eyre!("Failed to trigger workflow: {}", res.text()?));
        }

        Ok(())
    }
}
