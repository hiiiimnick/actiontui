use chrono::{DateTime, Local};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    pub id: u64,
    pub name: String,
    pub status: String,
    pub conclusion: Option<String>,
    pub workflow_id: u64,
    pub html_url: String,
    pub created_at: DateTime<Local>,
    pub display_title: String,
    pub head_branch: String,
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
            created_at: Local::now(),
            display_title: String::new(),
            head_branch: String::new(),
        }
    }
}
