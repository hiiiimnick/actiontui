#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct Workflow {
    pub id: u64,
    pub name: String,
    /// Location of the workflow file in the repository.
    pub path: String,
    pub state: String,
}
