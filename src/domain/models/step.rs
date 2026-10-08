use chrono::{DateTime, Duration, Utc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub name: String,
    pub status: String,
    pub conclusion: Option<String>,
    pub number: u64,
    /// `None` while the step has not started (queued, skipped).
    pub started_at: Option<DateTime<Utc>>,
    /// `None` while the step has not finished.
    pub completed_at: Option<DateTime<Utc>>,
}

impl Step {
    /// A skipped step never runs and therefore has no log output.
    pub fn is_skipped(&self) -> bool {
        self.conclusion.as_deref() == Some("skipped")
    }

    /// How long the step ran, or has been running up to `now`.
    /// `None` if it has not started.
    pub fn duration(&self, now: DateTime<Utc>) -> Option<Duration> {
        let started = self.started_at?;
        let end = self.completed_at.unwrap_or(now);
        Some((end - started).max(Duration::zero()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(s: &str) -> Option<DateTime<Utc>> {
        Some(DateTime::parse_from_rfc3339(s).unwrap().into())
    }

    fn step(started_at: Option<DateTime<Utc>>, completed_at: Option<DateTime<Utc>>) -> Step {
        Step {
            name: "s".into(),
            status: "completed".into(),
            conclusion: None,
            number: 1,
            started_at,
            completed_at,
        }
    }

    #[test]
    fn duration_of_finished_step() {
        let s = step(at("2026-01-01T10:00:00Z"), at("2026-01-01T10:01:30Z"));
        let now = at("2026-01-01T12:00:00Z").unwrap();
        assert_eq!(s.duration(now), Some(Duration::seconds(90)));
    }

    #[test]
    fn duration_of_running_step_uses_now() {
        let s = step(at("2026-01-01T10:00:00Z"), None);
        let now = at("2026-01-01T10:00:05Z").unwrap();
        assert_eq!(s.duration(now), Some(Duration::seconds(5)));
    }

    #[test]
    fn duration_of_unstarted_step_is_none() {
        let now = at("2026-01-01T10:00:05Z").unwrap();
        assert_eq!(step(None, None).duration(now), None);
    }

    #[test]
    fn duration_is_never_negative() {
        let s = step(at("2026-01-01T10:00:10Z"), None);
        let now = at("2026-01-01T10:00:00Z").unwrap();
        assert_eq!(s.duration(now), Some(Duration::zero()));
    }
}
