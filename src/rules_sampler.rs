use crate::{
    limiter::Limiter,
    priority_sampler::{PrioritySampler, SampleResult},
    sampling_priority::SamplingPriority,
    time_point::TimePoint,
    tools::{max_id_from_sample_rate, CONSTANT_RATE_HASH_FACTOR},
};
use eyre::Result;
use serde_json::Value;

pub(crate) struct RuleResult {
    pub matched: bool,
    pub rate: f64,
}

impl RuleResult {
    pub fn new() -> RuleResult {
        Self {
            matched: false,
            rate: std::f64::NAN,
        }
    }
}

pub(crate) struct RulesSampler<TimeProvider, RuleFunc>
where
    TimeProvider: Fn() -> TimePoint,
    RuleFunc: Fn(&str, &str) -> RuleResult,
{
    limiter: Limiter<TimeProvider>,
    sampling_rules: Vec<RuleFunc>,
    priority_sampler: PrioritySampler,
}

impl<TimeProvider, RuleFunc> RulesSampler<TimeProvider, RuleFunc>
where
    TimeProvider: Fn() -> TimePoint,
    RuleFunc: Fn(&str, &str) -> RuleResult,
{
    pub fn new(
        time_provider: TimeProvider,
        max_tokens: u64,
        refresh_rate: f64,
        tokens_per_refresh: u64,
    ) -> Self {
        Self {
            limiter: Limiter::<TimeProvider>::new(
                time_provider,
                max_tokens,
                refresh_rate,
                tokens_per_refresh,
            ),
            sampling_rules: Vec::new(),
            priority_sampler: PrioritySampler::new(),
        }
    }

    pub fn add_rule(&mut self, rule: RuleFunc) {
        self.sampling_rules.push(rule);
    }

    pub fn sample(
        &mut self,
        environment: &str,
        service: &str,
        name: &str,
        trace_id: u64,
    ) -> Result<SampleResult> {
        let rule_result = self.match_rule(service, name);
        if !rule_result.matched {
            return self.priority_sampler.sample(environment, service, trace_id);
        }

        let mut result = SampleResult::new();
        result.rule_rate = rule_result.rate;
        let max_hash = max_id_from_sample_rate(rule_result.rate);
        let hashed_id = trace_id as u128 * CONSTANT_RATE_HASH_FACTOR as u128;

        if hashed_id > max_hash as u128 {
            result.sampling_priority = Some(SamplingPriority::SamplerDrop);
            return Ok(result);
        }

        let limit_result = self.limiter.allow(1)?;
        result.limiter_rate = limit_result.effective_rate;
        result.sampling_priority = Some(if limit_result.allowed {
            SamplingPriority::SamplerKeep
        } else {
            SamplingPriority::SamplerDrop
        });
        Ok(result)
    }

    pub fn match_rule(&self, service: &str, name: &str) -> RuleResult {
        for rule in &self.sampling_rules {
            let result = rule(service, name);
            if result.matched {
                return result;
            }
        }

        RuleResult::new()
    }

    pub fn update_priority_sampler(&mut self, config: &Value) -> Result<()> {
        self.priority_sampler.configure(config)
    }
}
