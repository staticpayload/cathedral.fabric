//! Time types for CATHEDRAL.
//!
//! Uses logical time for determinism. Wall clock time is avoided.

use serde::{Deserialize, Serialize};

/// Logical time - monotonically increasing counter
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LogicalTime(u64);

impl LogicalTime {
    /// Create a new logical time at zero
    #[must_use]
    pub const fn zero() -> Self {
        Self(0)
    }

    /// Create from raw value
    #[must_use]
    pub const fn from_raw(value: u64) -> Self {
        Self(value)
    }

    /// Get raw value
    #[must_use]
    pub const fn as_u64(&self) -> u64 {
        self.0
    }

    /// Increment logical time
    pub fn increment(&mut self) {
        self.0 += 1;
    }

    /// Create incremented time
    #[must_use]
    pub fn incremented(&self) -> Self {
        Self(self.0 + 1)
    }

    /// Advance by n ticks
    pub fn advance(&mut self, n: u64) {
        self.0 += n;
    }
}

impl Default for LogicalTime {
    fn default() -> Self {
        Self::zero()
    }
}

impl std::fmt::Display for LogicalTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "T{}", self.0)
    }
}

impl From<u64> for LogicalTime {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

/// Wall clock timestamp - for metadata only, not for execution logic
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Timestamp {
    pub seconds: u64,
    pub nanos: u32,
}

impl Timestamp {
    /// Maximum nanoseconds per second
    pub const NANOS_PER_SEC: u32 = 1_000_000_000;

    /// Create a new timestamp
    #[must_use]
    pub fn new(seconds: u64, nanos: u32) -> Self {
        Self { seconds, nanos }
    }

    /// Get current timestamp (for metadata only)
    #[allow(clippy::missing_panics_doc)]
    pub fn now() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards");
        Self {
            seconds: duration.as_secs(),
            nanos: duration.subsec_nanos(),
        }
    }

    /// Convert to milliseconds
    #[must_use]
    pub const fn as_millis(&self) -> u128 {
        self.seconds as u128 * 1_000 + self.nanos as u128 / 1_000_000
    }

    /// Get duration since another timestamp
    #[must_use]
    pub fn duration_since(&self, earlier: &Timestamp) -> Duration {
        let mut seconds = self.seconds.saturating_sub(earlier.seconds);
        let mut nanos = self.nanos as i64 - earlier.nanos as i64;

        if nanos < 0 {
            seconds = seconds.saturating_sub(1);
            nanos += Self::NANOS_PER_SEC as i64;
        }

        Duration {
            seconds,
            nanos: nanos as u32,
        }
    }

    /// Add a duration
    #[must_use]
    pub fn add(&self, duration: &Duration) -> Self {
        let mut seconds = self.seconds + duration.seconds;
        let mut nanos = self.nanos + duration.nanos;

        if nanos >= Self::NANOS_PER_SEC {
            seconds += 1;
            nanos -= Self::NANOS_PER_SEC;
        }

        Self { seconds, nanos }
    }
}

impl std::fmt::Display for Timestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{:09}", self.seconds, self.nanos)
    }
}

/// A duration between timestamps
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Duration {
    pub seconds: u64,
    pub nanos: u32,
}

impl Duration {
    /// Create a new duration
    #[must_use]
    pub const fn new(seconds: u64, nanos: u32) -> Self {
        Self { seconds, nanos }
    }

    /// Zero duration
    #[must_use]
    pub const fn zero() -> Self {
        Self {
            seconds: 0,
            nanos: 0,
        }
    }

    /// Duration from seconds
    #[must_use]
    pub const fn from_secs(seconds: u64) -> Self {
        Self {
            seconds,
            nanos: 0,
        }
    }

    /// Duration from milliseconds
    #[must_use]
    pub const fn from_millis(millis: u64) -> Self {
        Self {
            seconds: millis / 1_000,
            nanos: ((millis % 1_000) * 1_000_000) as u32,
        }
    }

    /// Get total seconds
    #[must_use]
    pub const fn as_secs(&self) -> u64 {
        self.seconds
    }

    /// Get total milliseconds
    #[must_use]
    pub fn as_millis(&self) -> u128 {
        self.seconds as u128 * 1_000 + self.nanos as u128 / 1_000_000
    }

    /// Get total microseconds
    #[must_use]
    pub fn as_micros(&self) -> u128 {
        self.seconds as u128 * 1_000_000 + self.nanos as u128 / 1_000
    }

    /// Get total nanoseconds
    #[must_use]
    pub fn as_nanos(&self) -> u128 {
        self.seconds as u128 * 1_000_000_000 + self.nanos as u128
    }

    /// Saturating addition
    #[must_use]
    pub const fn saturating_add(&self, other: &Duration) -> Duration {
        let mut seconds = self.seconds.saturating_add(other.seconds);
        let mut nanos = self.nanos + other.nanos;

        if nanos >= Timestamp::NANOS_PER_SEC {
            seconds = seconds.saturating_add(1);
            nanos -= Timestamp::NANOS_PER_SEC;
        }

        Duration { seconds, nanos }
    }
}

impl Default for Duration {
    fn default() -> Self {
        Self::zero()
    }
}

impl std::fmt::Display for Duration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.seconds == 0 && self.nanos == 0 {
            write!(f, "0s")
        } else if self.seconds == 0 {
            write!(f, "{}ns", self.nanos)
        } else if self.nanos == 0 {
            write!(f, "{}s", self.seconds)
        } else {
            write!(f, "{}.{:09}s", self.seconds, self.nanos)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logical_time() {
        let t = LogicalTime::zero();
        assert_eq!(t.as_u64(), 0);

        let t2 = t.incremented();
        assert_eq!(t2.as_u64(), 1);
        assert_eq!(t.as_u64(), 0); // Original unchanged

        let mut t3 = t;
        t3.increment();
        assert_eq!(t3.as_u64(), 1);
    }

    #[test]
    fn test_logical_time_ord() {
        let t1 = LogicalTime::from_raw(1);
        let t2 = LogicalTime::from_raw(2);
        let t3 = LogicalTime::from_raw(2);

        assert!(t1 < t2);
        assert_eq!(t2, t3);
    }

    #[test]
    fn test_duration() {
        let d = Duration::from_secs(60);
        assert_eq!(d.as_secs(), 60);
        assert_eq!(d.as_millis(), 60_000);

        let d2 = Duration::from_millis(1500);
        assert_eq!(d2.as_secs(), 1);
        assert_eq!(d2.as_millis(), 1500);
    }

    #[test]
    fn test_timestamp_arithmetic() {
        let t1 = Timestamp::new(100, 500_000_000); // 100.5s
        let t2 = Timestamp::new(102, 200_000_000); // 102.2s

        let duration = t2.duration_since(&t1);
        assert_eq!(duration.seconds, 1);
        assert_eq!(duration.nanos, 700_000_000);

        let t3 = t1.add(&duration);
        assert_eq!(t3.seconds, 102);
        assert_eq!(t3.nanos, 200_000_000);
    }

    #[test]
    fn test_timestamp_nano_overflow() {
        let t = Timestamp::new(100, 900_000_000);
        let d = Duration::new(0, 200_000_000);

        let t2 = t.add(&d);
        assert_eq!(t2.seconds, 101);
        assert_eq!(t2.nanos, 100_000_000);
    }

    #[test]
    fn test_duration_saturating_add() {
        let d1 = Duration::new(u64::MAX, 500_000_000);
        let d2 = Duration::new(1, 600_000_000);

        let sum = d1.saturating_add(&d2);
        assert_eq!(sum.seconds, u64::MAX);
        assert_eq!(sum.nanos, 100_000_000);
    }
}
