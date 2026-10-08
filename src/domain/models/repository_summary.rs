use super::Repository;

/// A repository as listed among all the repositories of an account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositorySummary {
    pub repository: Repository,
    pub private: bool,
    pub description: Option<String>,
}
