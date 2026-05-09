#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct Workflow {
    pub id: u64,
    pub name: String,
    pub state: String,
}
