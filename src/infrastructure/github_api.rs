use crate::config::Account;
use crate::domain::models::Logs;
use crate::domain::models::{
    PendingDeployment, RateLimit, Repository, RepositorySummary, ReviewState, Run, Workflow,
    WorkflowInput,
};
use crate::domain::repositories::{RepositoryCatalog, WorkflowRepository};
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
    /// The status is not checked.
    fn send_unchecked(&self, request: RequestBuilder) -> Result<Response> {
        let response = request.send()?;
        if let Some(rate_limit) = parse_rate_limit(response.headers())
            && let Some(mut current) = self.rate_limit.lock().ok()
        {
            *current = Some(rate_limit);
        }
        Ok(response)
    }

    /// Like `send_unchecked`, but an unsuccessful status becomes an error
    /// carrying GitHub's message.
    fn send(&self, request: RequestBuilder) -> Result<Response> {
        let response = self.send_unchecked(request)?;
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

/// GitHub's maximum page size, and a safety limit on the number of pages.
const REPOSITORIES_PER_PAGE: usize = 100;
const MAX_REPOSITORY_PAGES: usize = 50;

impl RepositoryCatalog for HttpWorkflowRepository {
    fn list_repositories(&self) -> Result<Vec<RepositorySummary>> {
        let url = format!("{}/user/repos", self.account.api_base);
        let mut repositories = Vec::new();
        for page in 1..=MAX_REPOSITORY_PAGES {
            let request = self.get_request(url.clone()).query(&[
                ("per_page", REPOSITORIES_PER_PAGE.to_string()),
                ("page", page.to_string()),
                ("sort", "pushed".to_string()),
                ("direction", "desc".to_string()),
                (
                    "affiliation",
                    "owner,collaborator,organization_member".to_string(),
                ),
            ]);
            let body = self.send(request)?.text()?;
            let (found, listed) = parse_repositories(&body, &self.account.host)?;
            repositories.extend(found);
            if listed < REPOSITORIES_PER_PAGE {
                break;
            }
        }
        Ok(repositories)
    }
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
        let response = self.send_unchecked(self.get_request(url))?;
        // jobs that have not run (waiting for approval, skipped) or whose logs
        // expired have none: the download then answers 404
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(Logs::empty()?);
        }
        if !response.status().is_success() {
            let status = response.status();
            let path = response.url().path().to_string();
            let body = response.text().unwrap_or_default();
            return Err(color_eyre::eyre::eyre!(
                "{}",
                describe_api_error(status, &path, &body, &self.account.profile)
            ));
        }
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

    fn get_pending_deployments(
        &self,
        repo: &Repository,
        run_id: u64,
    ) -> Result<Vec<PendingDeployment>> {
        let url = format!(
            "{}/repos/{}/{}/actions/runs/{}/pending_deployments",
            self.account.api_base, repo.owner, repo.repo, run_id
        );
        let body = self.send(self.get_request(url))?.text()?;
        parse_pending_deployments(&body)
    }

    fn review_deployments(
        &self,
        repo: &Repository,
        run_id: u64,
        environment_ids: &[u64],
        state: ReviewState,
        comment: &str,
    ) -> Result<()> {
        let url = format!(
            "{}/repos/{}/{}/actions/runs/{}/pending_deployments",
            self.account.api_base, repo.owner, repo.repo, run_id
        );
        let body = review_body(environment_ids, state, comment);
        self.send(self.post_request(url).json(&body))?;
        Ok(())
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

fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A message for an unsuccessful response: the status, GitHub's own message and,
/// for the usual causes, a hint on how to fix it.
fn describe_api_error(status: StatusCode, path: &str, body: &str, profile: &str) -> String {
    let message = serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|json| json["message"].as_str().map(str::to_string))
        .unwrap_or_else(|| one_line(body).chars().take(200).collect());
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

#[derive(Deserialize)]
struct GithubPendingDeployment {
    environment: GithubEnvironment,
    current_user_can_approve: bool,
}

#[derive(Deserialize)]
struct GithubEnvironment {
    id: u64,
    name: String,
}

fn parse_pending_deployments(body: &str) -> Result<Vec<PendingDeployment>> {
    let deployments: Vec<GithubPendingDeployment> = serde_json::from_str(body)?;
    Ok(deployments
        .into_iter()
        .map(|d| PendingDeployment {
            environment_id: d.environment.id,
            environment: d.environment.name,
            can_approve: d.current_user_can_approve,
        })
        .collect())
}

/// The request body to approve or reject deployments.
fn review_body(environment_ids: &[u64], state: ReviewState, comment: &str) -> serde_json::Value {
    serde_json::json!({
        "environment_ids": environment_ids,
        "state": state.as_str(),
        "comment": comment,
    })
}

#[derive(Deserialize)]
struct GithubRepository {
    name: String,
    owner: GithubOwner,
    private: bool,
    #[serde(default)]
    archived: bool,
    #[serde(default)]
    disabled: bool,
    description: Option<String>,
}

#[derive(Deserialize)]
struct GithubOwner {
    login: String,
}

/// The usable repositories of one page and how many the page listed in total.
fn parse_repositories(body: &str, host: &str) -> Result<(Vec<RepositorySummary>, usize)> {
    let listed: Vec<GithubRepository> = serde_json::from_str(body)?;
    let count = listed.len();
    let usable = listed
        .into_iter()
        .filter(|r| !r.archived && !r.disabled)
        .map(|r| RepositorySummary {
            repository: Repository::new(host.to_string(), r.owner.login, r.name),
            private: r.private,
            description: r.description.filter(|d| !d.trim().is_empty()),
        })
        .collect();
    Ok((usable, count))
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
    fn multiline_bodies_become_one_line() {
        let body = "<?xml version=\"1.0\"?>\n<Error>\n  <Code>BlobNotFound</Code>\n</Error>";
        let text = describe_api_error(StatusCode::BAD_GATEWAY, "/x", body, "p");
        assert!(!text.contains('\n'), "{text}");
        assert!(
            text.contains("<Error> <Code>BlobNotFound</Code> </Error>"),
            "{text}"
        );
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
    fn parses_pending_deployments() {
        let body = r#"[
            {"environment": {"id": 161088068, "node_id": "x", "name": "staging", "url": "u"},
             "wait_timer": 30, "wait_timer_started_at": null,
             "current_user_can_approve": true,
             "reviewers": [{"type": "User", "reviewer": {"id": 1, "login": "octocat"}}]},
            {"environment": {"id": 7, "name": "prod"}, "current_user_can_approve": false,
             "reviewers": []}
        ]"#;
        let deployments = parse_pending_deployments(body).unwrap();
        assert_eq!(
            deployments,
            vec![
                PendingDeployment {
                    environment_id: 161088068,
                    environment: "staging".into(),
                    can_approve: true,
                },
                PendingDeployment {
                    environment_id: 7,
                    environment: "prod".into(),
                    can_approve: false,
                },
            ]
        );
        assert!(parse_pending_deployments("[]").unwrap().is_empty());
        assert!(parse_pending_deployments("{\"message\":\"x\"}").is_err());
    }

    #[test]
    fn builds_the_review_body() {
        let body = review_body(&[1, 2], ReviewState::Rejected, "not now");
        assert_eq!(
            body,
            serde_json::json!({
                "environment_ids": [1, 2],
                "state": "rejected",
                "comment": "not now",
            })
        );
    }

    #[test]
    fn parses_repositories_and_skips_unusable_ones() {
        let body = r#"[
            {"name": "tool", "full_name": "me/tool", "owner": {"login": "me"},
             "private": true, "archived": false, "disabled": false,
             "description": "A tool"},
            {"name": "old", "owner": {"login": "me"}, "private": false,
             "archived": true, "disabled": false, "description": null},
            {"name": "off", "owner": {"login": "me"}, "private": false,
             "archived": false, "disabled": true, "description": null},
            {"name": "api", "owner": {"login": "acme"}, "private": false,
             "description": "  "}
        ]"#;
        let (repositories, listed) = parse_repositories(body, "github.com").unwrap();
        assert_eq!(listed, 4);
        assert_eq!(repositories.len(), 2);
        assert_eq!(
            repositories[0].repository,
            Repository::new("github.com".into(), "me".into(), "tool".into())
        );
        assert!(repositories[0].private);
        assert_eq!(repositories[0].description.as_deref(), Some("A tool"));
        assert_eq!(repositories[1].repository.owner, "acme");
        assert_eq!(repositories[1].description, None);
    }

    #[test]
    fn repository_errors_are_errors() {
        assert!(parse_repositories(r#"{"message":"Bad credentials"}"#, "github.com").is_err());
        assert_eq!(parse_repositories("[]", "github.com").unwrap().1, 0);
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
