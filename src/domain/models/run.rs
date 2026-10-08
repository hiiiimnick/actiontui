use chrono::{DateTime, Local};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    pub id: u64,
    pub name: String,
    pub status: String,
    pub conclusion: Option<String>,
    pub workflow_id: u64,
    pub html_url: String,
    pub created_at: Option<DateTime<Local>>,
    pub display_title: String,
    pub head_branch: String,
}

impl Run {
    /// Whether the whole run can be run again: it has to be finished.
    pub fn can_rerun(&self) -> bool {
        self.status == "completed"
    }

    /// Whether the failed jobs of this run can be run again: the run has to be
    /// finished and unsuccessful because of a failing job.
    pub fn can_rerun_failed(&self) -> bool {
        self.status == "completed"
            && matches!(self.conclusion.as_deref(), Some("failure" | "timed_out"))
    }
}

impl Default for Run {
    fn default() -> Self {
        Self {
            id: 0,
            name: String::new(),
            status: String::new(),
            conclusion: None,
            workflow_id: 0,
            html_url: String::new(),
            created_at: Some(Local::now()),
            display_title: String::new(),
            head_branch: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(status: &str, conclusion: Option<&str>) -> Run {
        Run {
            status: status.into(),
            conclusion: conclusion.map(Into::into),
            ..Run::default()
        }
    }

    #[test]
    fn failed_runs_can_be_rerun() {
        assert!(run("completed", Some("failure")).can_rerun_failed());
        assert!(run("completed", Some("timed_out")).can_rerun_failed());
    }

    #[test]
    fn only_finished_runs_can_be_rerun_as_a_whole() {
        assert!(run("completed", Some("success")).can_rerun());
        assert!(run("completed", Some("cancelled")).can_rerun());
        assert!(!run("in_progress", None).can_rerun());
        assert!(!run("queued", None).can_rerun());
    }

    #[test]
    fn other_runs_cannot() {
        assert!(!run("completed", Some("success")).can_rerun_failed());
        assert!(!run("completed", Some("cancelled")).can_rerun_failed());
        assert!(!run("completed", Some("skipped")).can_rerun_failed());
        assert!(!run("completed", None).can_rerun_failed());
        assert!(!run("in_progress", None).can_rerun_failed());
        assert!(!run("queued", None).can_rerun_failed());
    }
}
