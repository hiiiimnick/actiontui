use chrono::{DateTime, Local};

use crate::domain::Step;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Job {
    pub id: u64,
    pub name: String,
    pub status: String,
    pub conclusion: Option<String>,
    pub started_at: DateTime<Local>,
    pub completed_at: DateTime<Local>,
    pub steps: Vec<Step>,
}
