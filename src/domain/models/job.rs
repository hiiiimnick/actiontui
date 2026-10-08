use chrono::{DateTime, Local};

use crate::domain::Step;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Job {
    pub id: u64,
    pub name: String,
    pub status: String,
    pub conclusion: Option<String>,
    pub started_at: Option<DateTime<Local>>,
    pub completed_at: Option<DateTime<Local>>,
    pub steps: Vec<Step>,
}

impl Job {
    /// Whether this job can be run again: it has to be finished.
    pub fn can_rerun(&self) -> bool {
        self.status == "completed"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn job(status: &str) -> Job {
        Job {
            id: 1,
            name: "build".into(),
            status: status.into(),
            conclusion: None,
            started_at: None,
            completed_at: None,
            steps: Vec::new(),
        }
    }

    #[test]
    fn only_finished_jobs_can_be_rerun() {
        assert!(job("completed").can_rerun());
        assert!(!job("in_progress").can_rerun());
        assert!(!job("queued").can_rerun());
    }
}
