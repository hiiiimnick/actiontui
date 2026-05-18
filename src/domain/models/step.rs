#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub name: String,
    pub status: String,
    pub conclusion: Option<String>,
    pub number: u64,
    pub started_at: String,
    pub completed_at: String,
}
