/// An environment a run is waiting to deploy to until it is approved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingDeployment {
    pub environment_id: u64,
    pub environment: String,
    /// Whether the current user is one of the environment's required reviewers.
    pub can_approve: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewState {
    Approved,
    Rejected,
}

impl ReviewState {
    /// The name GitHub uses for the state.
    pub fn as_str(&self) -> &'static str {
        match self {
            ReviewState::Approved => "approved",
            ReviewState::Rejected => "rejected",
        }
    }

    pub fn verb(&self) -> &'static str {
        match self {
            ReviewState::Approved => "Approve",
            ReviewState::Rejected => "Reject",
        }
    }
}

/// The deployments the current user may review, `None` if there are none.
pub fn reviewable(pending: &[PendingDeployment]) -> Option<Vec<&PendingDeployment>> {
    let reviewable: Vec<&PendingDeployment> = pending.iter().filter(|d| d.can_approve).collect();
    (!reviewable.is_empty()).then_some(reviewable)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deployment(id: u64, name: &str, can_approve: bool) -> PendingDeployment {
        PendingDeployment {
            environment_id: id,
            environment: name.into(),
            can_approve,
        }
    }

    #[test]
    fn review_state_names() {
        assert_eq!(ReviewState::Approved.as_str(), "approved");
        assert_eq!(ReviewState::Rejected.as_str(), "rejected");
        assert_eq!(ReviewState::Approved.verb(), "Approve");
        assert_eq!(ReviewState::Rejected.verb(), "Reject");
    }

    #[test]
    fn only_deployments_the_user_may_review_are_offered() {
        let pending = [deployment(1, "int", false), deployment(2, "prod", true)];
        let reviewable = reviewable(&pending).unwrap();
        assert_eq!(reviewable.len(), 1);
        assert_eq!(reviewable[0].environment, "prod");
    }

    #[test]
    fn nothing_to_review() {
        assert!(reviewable(&[]).is_none());
        assert!(reviewable(&[deployment(1, "prod", false)]).is_none());
    }
}
