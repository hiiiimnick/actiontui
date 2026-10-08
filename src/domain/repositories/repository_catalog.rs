use std::fmt::Debug;

use crate::domain::models::RepositorySummary;
use color_eyre::Result;

/// The repositories an account has access to.
pub trait RepositoryCatalog: Debug {
    /// All repositories the account can see, most recently active first.
    /// Archived and disabled repositories are left out, they cannot run workflows.
    fn list_repositories(&self) -> Result<Vec<RepositorySummary>>;
}
