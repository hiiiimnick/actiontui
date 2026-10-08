use crate::config::Account;
use crate::domain::models::Logs;
use crate::domain::models::{RateLimit, Repository, Run, Workflow, WorkflowInput};
use crate::domain::repositories::WorkflowRepository;
use crate::domain::{Job, Step};
use crate::infrastructure::map_optional_time;
use crate::infrastructure::workflow_definition::parse_dispatch_inputs;
use chrono::{DateTime, Utc};
use color_eyre::Result;
use color_eyre::eyre::Ok;
use reqwest::StatusCode;
use reqwest::blocking::{Client, RequestBuilder, Response};
use reqwest::header::{ACCEPT, AUTHORIZATION, HeaderMap, USER_AGENT};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug)]
pub struct HttpWorkflowRepository {
    account: Account,
    client: Client,
    /// From the headers of the most recent response.
    rate_limit: Mutex<Option<RateLimit>>,
}

impl HttpWorkflowRepository {
    pub fn new(account: Account) -> Self {
        Self {
            account,
            client: Client::new(),
            rate_limit: Mutex::new(None),
        }
    }

    /// Sends the request and remembers the API quota reported in the response.
    /// An unsuccessful status becomes an error carrying GitHub's message.
    fn send(&self, request: RequestBuilder) -> Result<Response> {
        let response = request.send()?;
        if let Some(rate_limit) = parse_rate_limit(response.headers())
            && let Some(mut current) = self.rate_limit.lock().ok()
        {
            *current = Some(rate_limit);
        }
        if response.status().is_success() {
            return Ok(response);
        }

        let status = response.status();
        let path = response.url().path().to_string();
        let body = response.text().unwrap_or_default();
        Err(color_eyre::eyre::eyre!(
            "{}",
            describe_api_error(status, &path, &body, &self.account.profile)
        ))
    }

    fn post_request(&self, url: String) -> reqwest::blocking::RequestBuilder {
        self.client
            .post(url)
            .header(USER_AGENT, "actiontui")
            .header(AUTHORIZATION, format!("Bearer {}", self.account.token))
            .header(ACCEPT, "application/vnd.github+json")
    }

    /// POSTs without a body.
    fn post_empty(&self, url: String) -> Result<()> {
        self.send(self.post_request(url))?;
        Ok(())
    }

    fn get_request(&self, url: String) -> reqwest::blocking::RequestBuilder {
        self.client
            .get(url)
            .header(USER_AGENT, "actiontui")
            .header(AUTHORIZATION, format!("Bearer {}", self.account.token))
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
    fn rate_limit(&self) -> Option<RateLimit> {
        self.rate_limit.lock().ok().and_then(|current| *current)
    }

    fn get_workflows(&self, repo: &Repository) -> Result<Vec<Workflow>> {
        let url = format!(
            "{}/repos/{}/{}/actions/workflows",
            self.account.api_base, repo.owner, repo.repo
        );

        let response: GithubWorkflowResponse = self.send(self.get_request(url))?.json()?;

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
            "{}/repos/{}/{}/actions/workflows/{}/runs",
            self.account.api_base, repo.owner, repo.repo, workflow_id
        );

        let response: GithubRunResponse = self.send(self.get_request(url))?.json()?;

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
            "{}/repos/{}/{}/actions/runs/{}/jobs",
            self.account.api_base, repo.owner, repo.repo, run_id
        );

        let response: GithubJobResponse = self.send(self.get_request(url))?.json()?;

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
            "{}/repos/{}/{}/actions/jobs/{}",
            self.account.api_base, repo.owner, repo.repo, job_id
        );

        let response: GithubWorkflowRunJob = self.send(self.get_request(url))?.json()?;
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
            "{}/repos/{}/{}/actions/jobs/{}/logs",
            self.account.api_base, repo.owner, repo.repo, job_id
        );
        let response = self.send(self.get_request(url))?;
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
            "{}/repos/{}/{}/contents/{}",
            self.account.api_base,
            repo.owner,
            repo.repo,
            workflow.path.trim_start_matches('/')
        );

        // the raw media type returns the file itself instead of base64 JSON
        let request = self
            .get_request(url)
            .query(&[("ref", reference)])
            .header(ACCEPT, "application/vnd.github.raw+json");
        let yaml = self.send(request)?.text()?;
        parse_dispatch_inputs(&yaml)
    }

    fn rerun_run(&self, repo: &Repository, run_id: u64) -> Result<()> {
        self.post_empty(format!(
            "{}/repos/{}/{}/actions/runs/{}/rerun",
            self.account.api_base, repo.owner, repo.repo, run_id
        ))
    }

    fn rerun_job(&self, repo: &Repository, job_id: u64) -> Result<()> {
        self.post_empty(format!(
            "{}/repos/{}/{}/actions/jobs/{}/rerun",
            self.account.api_base, repo.owner, repo.repo, job_id
        ))
    }

    fn rerun_failed_jobs(&self, repo: &Repository, run_id: u64) -> Result<()> {
        self.post_empty(format!(
            "{}/repos/{}/{}/actions/runs/{}/rerun-failed-jobs",
            self.account.api_base, repo.owner, repo.repo, run_id
        ))
    }

    fn trigger_workflow(
        &self,
        repo: &Repository,
        workflow_id: u64,
        reference: &str,
        inputs: &HashMap<String, String>,
    ) -> Result<()> {
        let url = format!(
            "{}/repos/{}/{}/actions/workflows/{}/dispatches",
            self.account.api_base, repo.owner, repo.repo, workflow_id
        );

        let mut body = serde_json::json!({
            "ref": reference
        });
        if !inputs.is_empty() {
            body["inputs"] = serde_json::json!(inputs);
        }

        self.send(self.post_request(url).json(&body))?;

        Ok(())
    }
}

/// A message for an unsuccessful response: the status, GitHub's own message and,
/// for the usual causes, a hint on how to fix it.
fn describe_api_error(status: StatusCode, path: &str, body: &str, profile: &str) -> String {
    let message = serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|json| json["message"].as_str().map(str::to_string))
        .unwrap_or_else(|| body.trim().chars().take(200).collect());
    let hint = match status.as_u16() {
        401 => format!(" (the token of profile '{profile}' is invalid or expired)"),
        403 => format!(
            " (the token of profile '{profile}' lacks permission, or it still has to be \
             authorized for SAML SSO)"
        ),
        404 => format!(
            " (not found, or the token of profile '{profile}' cannot access it: for \
             organizations with SSO authorize the token, fine-grained tokens need the \
             organization as resource owner)"
        ),
        _ => String::new(),
    };
    format!("GitHub returned {status} for {path}: {message}{hint}")
}

/// Reads GitHub's `X-RateLimit-*` response headers.
fn parse_rate_limit(headers: &HeaderMap) -> Option<RateLimit> {
    let number = |name: &str| headers.get(name)?.to_str().ok()?.parse::<i64>().ok();
    Some(RateLimit {
        limit: number("x-ratelimit-limit")?.try_into().ok()?,
        remaining: number("x-ratelimit-remaining")?.try_into().ok()?,
        reset_at: number("x-ratelimit-reset").and_then(|t| DateTime::from_timestamp(t, 0)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers(pairs: &[(&'static str, &'static str)]) -> HeaderMap {
        let mut map = HeaderMap::new();
        for (name, value) in pairs {
            map.insert(*name, value.parse().unwrap());
        }
        map
    }

    #[test]
    fn api_errors_show_status_message_and_hint() {
        let text = describe_api_error(
            StatusCode::NOT_FOUND,
            "/repos/acme/tool/actions/workflows",
            r#"{"message":"Not Found","documentation_url":"https://docs.github.com/rest","status":"404"}"#,
            "work",
        );
        assert!(text.contains("404 Not Found"), "{text}");
        assert!(
            text.contains("/repos/acme/tool/actions/workflows"),
            "{text}"
        );
        assert!(text.contains("Not Found"), "{text}");
        assert!(
            text.contains("profile 'work'") && text.contains("SSO"),
            "{text}"
        );
    }

    #[test]
    fn api_errors_without_json_use_the_body() {
        let text = describe_api_error(StatusCode::BAD_GATEWAY, "/x", "  upstream down \n", "p");
        assert!(text.ends_with(": upstream down"), "{text}");
    }

    #[test]
    fn unauthorized_hints_at_the_token() {
        let text = describe_api_error(
            StatusCode::UNAUTHORIZED,
            "/x",
            r#"{"message":"Bad credentials"}"#,
            "personal",
        );
        assert!(
            text.contains("Bad credentials") && text.contains("invalid or expired"),
            "{text}"
        );
    }

    #[test]
    fn parses_rate_limit_headers() {
        let rate = parse_rate_limit(&headers(&[
            ("x-ratelimit-limit", "5000"),
            ("x-ratelimit-remaining", "4321"),
            ("x-ratelimit-reset", "1700000000"),
        ]))
        .unwrap();
        assert_eq!(rate.limit, 5000);
        assert_eq!(rate.remaining, 4321);
        assert_eq!(rate.reset_at, DateTime::from_timestamp(1_700_000_000, 0));
    }

    #[test]
    fn missing_or_invalid_headers_give_none() {
        assert_eq!(parse_rate_limit(&headers(&[])), None);
        assert_eq!(
            parse_rate_limit(&headers(&[
                ("x-ratelimit-limit", "5000"),
                ("x-ratelimit-remaining", "lots"),
            ])),
            None
        );
    }

    #[test]
    fn reset_is_optional() {
        let rate = parse_rate_limit(&headers(&[
            ("x-ratelimit-limit", "60"),
            ("x-ratelimit-remaining", "59"),
        ]))
        .unwrap();
        assert_eq!(rate.reset_at, None);
    }
}
