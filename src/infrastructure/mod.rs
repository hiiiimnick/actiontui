pub mod github_api;

use chrono::{DateTime, Local, Utc};
pub use github_api::HttpWorkflowRepository;

pub fn map_optional_time(optional_time: Option<DateTime<Utc>>) -> Option<DateTime<Local>> {
    if let Some(time) = optional_time {
        return Option::from(DateTime::from(time));
    } else {
        return None;
    }
}
