#[cfg(test)]
use mock_instant::Instant;
#[cfg(not(test))]
use std::time::Instant;
use std::time::SystemTime;

#[derive(Clone)]
pub(crate) struct TimePoint {
    pub absolute_time: SystemTime,
    pub relative_time: Instant,
}

impl TimePoint {
    pub fn new() -> TimePoint {
        Self {
            absolute_time: SystemTime::now(),
            relative_time: Instant::now(),
        }
    }
}
