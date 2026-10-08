use chrono::{DateTime, Duration, Utc};

/// How much of the GitHub API quota is left.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RateLimit {
    pub limit: u32,
    pub remaining: u32,
    /// When the quota is renewed.
    pub reset_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitLevel {
    Ok,
    /// 10% or less of the quota is left.
    Low,
    Exhausted,
}

impl RateLimit {
    /// The part of the quota that is used up, 0.0 to 100.0.
    pub fn used_percent(&self) -> f64 {
        if self.limit == 0 {
            return 0.0;
        }
        let used = self.limit.saturating_sub(self.remaining);
        f64::from(used) / f64::from(self.limit) * 100.0
    }

    pub fn level(&self) -> RateLimitLevel {
        if self.remaining == 0 {
            RateLimitLevel::Exhausted
        } else if u64::from(self.remaining) * 10 <= u64::from(self.limit) {
            RateLimitLevel::Low
        } else {
            RateLimitLevel::Ok
        }
    }

    /// Time until the quota is renewed, `None` if unknown or already past.
    pub fn time_until_reset(&self, now: DateTime<Utc>) -> Option<Duration> {
        self.reset_at
            .map(|reset| reset - now)
            .filter(|left| *left > Duration::zero())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limit(remaining: u32) -> RateLimit {
        RateLimit {
            limit: 5000,
            remaining,
            reset_at: None,
        }
    }

    #[test]
    fn levels() {
        assert_eq!(limit(4000).level(), RateLimitLevel::Ok);
        assert_eq!(limit(501).level(), RateLimitLevel::Ok);
        assert_eq!(limit(500).level(), RateLimitLevel::Low);
        assert_eq!(limit(1).level(), RateLimitLevel::Low);
        assert_eq!(limit(0).level(), RateLimitLevel::Exhausted);
    }

    #[test]
    fn used_percent() {
        assert_eq!(limit(5000).used_percent(), 0.0);
        assert_eq!(limit(4500).used_percent(), 10.0);
        assert_eq!(limit(0).used_percent(), 100.0);
        // more remaining than the limit never gives a negative value
        assert_eq!(limit(6000).used_percent(), 0.0);
        let unlimited = RateLimit {
            limit: 0,
            remaining: 0,
            reset_at: None,
        };
        assert_eq!(unlimited.used_percent(), 0.0);
    }

    #[test]
    fn time_until_reset() {
        let now = DateTime::from_timestamp(1_000, 0).unwrap();
        let mut rate = limit(10);
        assert_eq!(rate.time_until_reset(now), None);

        rate.reset_at = DateTime::from_timestamp(1_600, 0);
        assert_eq!(rate.time_until_reset(now), Some(Duration::seconds(600)));

        rate.reset_at = DateTime::from_timestamp(900, 0);
        assert_eq!(rate.time_until_reset(now), None);
    }
}
