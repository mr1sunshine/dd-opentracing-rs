use crate::dd::time_point::TimePoint;
use eyre::{eyre, Result};
#[cfg(test)]
use mock_instant::Instant;
use std::sync::Mutex;
use std::time::Duration;
#[cfg(not(test))]
use std::time::Instant;

pub(crate) struct LimitResult {
    pub allowed: bool,
    pub effective_rate: f64,
}

#[derive(Derivative)]
#[derivative(Debug)]
struct LimitData<F>
where
    F: Fn() -> TimePoint,
{
    #[derivative(Debug = "ignore")]
    pub time_provider: F,

    pub num_tokens: u64,
    pub max_tokens: u64,
    pub refresh_interval: Duration,
    pub tokens_per_refresh: u64,
    pub next_refresh: Instant,
    pub previous_rates: Vec<f64>,
    pub previous_rates_sum: f64,
    pub current_period: Instant,
    pub num_allowed: u64,
    pub num_requested: u64,
}

impl<F> LimitData<F>
where
    F: Fn() -> TimePoint,
{
    pub fn new(
        time_provider: F,
        max_tokens: u64,
        refresh_rate: f64,
        tokens_per_refresh: u64,
    ) -> Self {
        let previous_rates = vec![1.0; 9];
        let refresh_interval = Duration::from_secs(1)
            .div_f64(refresh_rate)
            .mul_f64(tokens_per_refresh as f64);
        let now = time_provider();
        let next_refresh = now.relative_time + refresh_interval;
        let current_period = now.relative_time;
        let previous_rates_sum = previous_rates.iter().sum();
        Self {
            time_provider,
            num_tokens: max_tokens,
            max_tokens,
            refresh_interval,
            tokens_per_refresh,
            next_refresh,
            previous_rates,
            previous_rates_sum,
            current_period,
            num_allowed: 0,
            num_requested: 0,
        }
    }
}

#[derive(Debug)]
pub(crate) struct Limiter<F>
where
    F: Fn() -> TimePoint,
{
    data: Mutex<LimitData<F>>,
}

impl<F> Limiter<F>
where
    F: Fn() -> TimePoint,
{
    pub fn new(
        time_provider: F,
        max_tokens: u64,
        refresh_rate: f64,
        tokens_per_refresh: u64,
    ) -> Self {
        Self {
            data: Mutex::new(LimitData::new(
                time_provider,
                max_tokens,
                refresh_rate,
                tokens_per_refresh,
            )),
        }
    }

    pub fn allow(&mut self, tokens_requested: u64) -> Result<LimitResult> {
        let mut data = self.data.lock().map_err(|_| eyre!("mutex lock failed"))?;
        let now = (data.time_provider)();
        let intervals = (now.relative_time - data.current_period).as_secs() as usize;
        if intervals > 0 {
            if intervals >= data.previous_rates.len() {
                for i in data.previous_rates[1..].iter_mut() {
                    *i = 1.0;
                }
            } else {
                let back = data.previous_rates[0..data.previous_rates.len() - intervals].to_vec();
                data.previous_rates.truncate(intervals);
                data.previous_rates.extend_from_slice(&back);
                if data.num_requested > 0 {
                    data.previous_rates[intervals - 1] =
                        data.num_allowed as f64 / data.num_requested as f64;
                } else {
                    data.previous_rates[intervals - 1] = 1.0;
                }
                if intervals > 2 {
                    for i in data.previous_rates[0..intervals - 2].iter_mut() {
                        *i = 1.0;
                    }
                }
            }
            data.previous_rates_sum = data.previous_rates.iter().sum();
            data.num_allowed = 0;
            data.num_requested = 0;
            data.current_period = now.relative_time;
        }

        data.num_requested += 1;
        // refill tokens
        if now.relative_time >= data.next_refresh {
            let intervals = (now.relative_time - data.next_refresh).as_nanos()
                / data.refresh_interval.as_nanos()
                + 1;
            if intervals > 0 {
                let duration = Duration::from_nanos(
                    data.refresh_interval.as_nanos() as u64 * intervals as u64,
                );
                data.next_refresh += duration;
                data.num_tokens = intervals as u64 * data.tokens_per_refresh;
                if data.num_tokens > data.max_tokens {
                    data.num_tokens = data.max_tokens;
                }
            }
        }

        let allowed = if data.num_tokens >= tokens_requested {
            data.num_allowed += 1;
            data.num_tokens -= tokens_requested;
            true
        } else {
            false
        };

        let effective_rate = (data.previous_rates_sum
            + data.num_allowed as f64 / data.num_requested as f64)
            / (data.previous_rates.len() + 1) as f64;
        Ok(LimitResult {
            allowed,
            effective_rate,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::time::UNIX_EPOCH;

    use super::*;
    use crate::time_point::TimePoint;
    use mock_instant::MockClock;

    #[test]
    fn limits_requested() {
        let time_provider = || TimePoint {
            absolute_time: UNIX_EPOCH,
            relative_time: Instant::now(),
        };
        let mut limiter = Limiter::new(time_provider, 1, 1.0, 1);
        let first = limiter.allow(1).unwrap();
        let second = limiter.allow(1).unwrap();
        assert!(first.allowed);
        assert!(!second.allowed);
    }

    #[test]
    fn refreshes_over_time() {
        let time_provider = || TimePoint {
            absolute_time: UNIX_EPOCH,
            relative_time: Instant::now(),
        };
        let mut limiter = Limiter::new(time_provider, 1, 1.0, 1);
        let first = limiter.allow(1).unwrap();
        let second = limiter.allow(1).unwrap();
        MockClock::advance(Duration::from_secs(1));
        let third = limiter.allow(1).unwrap();
        assert!(first.allowed);
        assert!(!second.allowed);
        assert!(third.allowed);
    }

    #[test]
    fn handles_long_intervals() {
        let time_provider = || TimePoint {
            absolute_time: UNIX_EPOCH,
            relative_time: Instant::now(),
        };
        let mut limiter = Limiter::new(time_provider, 1, 1.0, 1);
        let first = limiter.allow(1).unwrap();
        MockClock::advance(Duration::from_secs(2));
        let second = limiter.allow(1).unwrap();
        let third = limiter.allow(1).unwrap();
        assert!(first.allowed);
        assert!(second.allowed);
        assert!(!third.allowed);
    }

    #[test]
    fn calculates_effective_rate() {
        let time_provider = || TimePoint {
            absolute_time: UNIX_EPOCH,
            relative_time: Instant::now(),
        };
        let mut limiter = Limiter::new(time_provider, 1, 1.0, 1);
        let first = limiter.allow(1).unwrap();
        assert!(first.allowed);
        assert_eq!(first.effective_rate, 1.0);
        let second = limiter.allow(1).unwrap();
        assert!(!second.allowed);
        assert_eq!(second.effective_rate, 0.95);
        MockClock::advance(Duration::from_secs(10));
        let third = limiter.allow(1).unwrap();
        assert!(third.allowed);
        assert_eq!(third.effective_rate, 1.0);
    }

    #[test]
    fn updates_tokens_at_sub_second_intervals() {
        let time_provider = || TimePoint {
            absolute_time: UNIX_EPOCH,
            relative_time: Instant::now(),
        };
        let mut limiter = Limiter::new(time_provider, 5, 5.0, 1);
        for _ in 0..5 {
            let result = limiter.allow(1).unwrap();
            assert!(result.allowed);
        }
        let all_consumed = limiter.allow(1).unwrap();
        assert!(!all_consumed.allowed);

        MockClock::advance(Duration::from_millis(200));

        let first = limiter.allow(1).unwrap();
        assert!(first.allowed);
        let second = limiter.allow(1).unwrap();
        assert!(!second.allowed);

        MockClock::advance(Duration::from_secs(1));
        for _ in 0..5 {
            let result = limiter.allow(1).unwrap();
            assert!(result.allowed);
        }
        let all_consumed = limiter.allow(1).unwrap();
        assert!(!all_consumed.allowed);
    }
}
