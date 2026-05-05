use serde::Deserialize;

#[derive(Debug)]
pub struct Repository {
    pub owner: String,
    pub repo: String,
}

#[derive(Debug, Deserialize)]
pub struct Workflow {
    pub id: u64,
    pub name: String,
    pub state: String,
}

#[derive(Debug, Deserialize)]
pub struct WorkflowResponse {
    pub total_count: u32,
    pub workflows: Vec<Workflow>,
}
