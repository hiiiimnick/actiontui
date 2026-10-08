/// An input a workflow asks for when it is started manually
/// (`on.workflow_dispatch.inputs`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowInput {
    pub name: String,
    pub description: Option<String>,
    pub required: bool,
    pub default: Option<String>,
    pub kind: InputKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputKind {
    Text,
    Number,
    Boolean,
    Choice(Vec<String>),
}
