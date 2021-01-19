pub(crate) const CONSTANT_RATE_HASH_FACTOR: u64 = 1111111111111111111;

const MAX_TRACE_ID_DOUBLE: f64 = std::u64::MAX as f64;

pub(crate) fn max_id_from_sample_rate(rate: f64) -> u64 {
    if rate == 1.0 {
        std::u64::MAX
    } else if rate > 0.0 {
        (rate * MAX_TRACE_ID_DOUBLE) as u64
    } else {
        0
    }
}
