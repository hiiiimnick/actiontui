use reqwest::{
    blocking::Client,
    header::{ACCEPT, AUTHORIZATION, USER_AGENT},
};

use crate::{Config, domain::Repository, domain::WorkflowResponse};

pub fn get_workflows(cfg: &Config, repo: Repository) -> Result<WorkflowResponse, reqwest::Error> {
    let request_url = format!(
        "https://api.{base_url}/repos/{owner}/{repo}/actions/workflows",
        base_url = cfg.url,
        owner = repo.owner,
        repo = repo.repo
    );

    let client = Client::new()
        .get(request_url)
        .header(USER_AGENT, "workflow-tui-api-client")
        .header(AUTHORIZATION, format!("Bearer {}", cfg.pat))
        .header(ACCEPT, "application/vnd.github+json")
        .send();

    match client {
        Ok(result) => {
            let workflow_response: WorkflowResponse = result.json()?;
            Ok(workflow_response)
        }
        Err(error) => Err(error),
    }
}
