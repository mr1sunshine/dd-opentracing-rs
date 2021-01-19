use super::SamplingPriority;
use crate::dd::utils::{max_id_from_sample_rate, CONSTANT_RATE_HASH_FACTOR};
use eyre::{eyre, Result};
use serde_json::Value;
use std::{collections::HashMap, sync::Mutex};

const PRIORITY_SAMPLER_DEFAULT_RATE_KEY: &str = "service:,env:";

#[derive(Default, Debug)]
pub struct SampleResult {
    pub rule_rate: f64,
    pub limiter_rate: f64,
    pub priority_rate: f32,
    pub sampling_priority: Option<SamplingPriority>,
}

impl SampleResult {
    pub fn new() -> SampleResult {
        Self {
            rule_rate: std::f64::NAN,
            limiter_rate: std::f64::NAN,
            priority_rate: std::f32::NAN,
            sampling_priority: None,
        }
    }
}

#[derive(Default, Clone, Debug)]
pub struct SamplingRate {
    pub rate: f32,
    pub max_hash: u64,
}

#[derive(Debug)]
struct PrioritySamplerData {
    pub agent_sampling_rates: HashMap<String, SamplingRate>,
    pub default_sampling_rate: SamplingRate,
}

#[derive(Debug)]
pub struct PrioritySampler {
    data: Mutex<PrioritySamplerData>,
}

impl PrioritySampler {
    pub fn new() -> Self {
        let data = PrioritySamplerData {
            agent_sampling_rates: HashMap::new(),
            default_sampling_rate: SamplingRate {
                rate: 1.0,
                max_hash: std::u64::MAX,
            },
        };
        Self {
            data: Mutex::new(data),
        }
    }

    pub fn sample(&self, environment: &str, service: &str, trace_id: u64) -> Result<SampleResult> {
        let data = self.data.lock().map_err(|_| eyre!("mutex lock failed"))?;

        let mut applied_rate = data.default_sampling_rate.clone();

        let key = format!("service:{},env:{}", service, environment);
        if let Some(rule) = data.agent_sampling_rates.get(&key) {
            applied_rate = rule.clone();
        }

        let hashed_id = trace_id as u128 * CONSTANT_RATE_HASH_FACTOR as u128;
        let sampling_priority = if hashed_id as u64 >= applied_rate.max_hash {
            Some(SamplingPriority::SamplerDrop)
        } else {
            Some(SamplingPriority::SamplerKeep)
        };

        Ok(SampleResult {
            priority_rate: applied_rate.rate,
            sampling_priority,
            ..Default::default()
        })
    }

    pub fn configure(&mut self, config: &Value) -> Result<()> {
        let mut rates = HashMap::new();
        let object = if let Value::Object(object) = config {
            object
        } else {
            return Err(eyre!("Invalid json for config. Expected Object."));
        };

        let mut data = self.data.lock().map_err(|_| eyre!("mutex lock failed"))?;

        for (key, value) in object {
            let rate = if let Value::Number(value) = value {
                value.as_f64().ok_or(eyre!("No float"))?
            } else {
                return Err(eyre!(
                    "Invalid json for config. All values should be numbers."
                ));
            };

            let new_rate = SamplingRate {
                rate: rate as f32,
                max_hash: max_id_from_sample_rate(rate),
            };
            if key == PRIORITY_SAMPLER_DEFAULT_RATE_KEY {
                data.default_sampling_rate = new_rate;
            } else {
                rates.insert(key.clone(), new_rate);
            }
        }
        data.agent_sampling_rates = rates;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unconfigured() {
        let sampler = PrioritySampler::new();
        let result = sampler.sample("", "", 0).unwrap();
        assert_eq!(result.priority_rate, 1.0);
        assert_eq!(
            result.sampling_priority,
            Some(SamplingPriority::SamplerKeep)
        );
        let result = sampler.sample("env", "service", 1).unwrap();
        assert_eq!(result.priority_rate, 1.0);
        assert_eq!(
            result.sampling_priority,
            Some(SamplingPriority::SamplerKeep)
        );
    }

    mod configured {
        use super::*;
        use mersenne_twister::MersenneTwister;
        use rand::Rng;

        const CONFIG_JSON: &str = r#"
        {
            "service:nginx,env:": 0.8,
            "service:nginx,env:prod": 0.2
        }"#;

        #[test]
        fn spans_dont_match() {
            let config: Value = serde_json::from_str(CONFIG_JSON).unwrap();
            let mut sampler = PrioritySampler::new();
            sampler.configure(&config).unwrap();
            let result = sampler
                .sample("different env", "different service", 1)
                .unwrap();
            assert_eq!(result.priority_rate, 1.0);
            assert_eq!(
                result.sampling_priority,
                Some(SamplingPriority::SamplerKeep)
            );
        }

        fn sampled_test(environment: &str, service: &str) -> f32 {
            let mut rng: MersenneTwister = Default::default();
            let config: Value = serde_json::from_str(CONFIG_JSON).unwrap();
            let mut sampler = PrioritySampler::new();
            sampler.configure(&config).unwrap();

            // Case 1, service:nginx,env: => 0.8
            let mut count_sampled = 0;
            let total = 100000;
            for _ in 0..total {
                let result = sampler
                    .sample(environment, service, rng.next_u64())
                    .unwrap();
                assert!(
                    (result.sampling_priority == Some(SamplingPriority::SamplerKeep)
                        || result.sampling_priority == Some(SamplingPriority::SamplerDrop))
                );
                count_sampled += if result.sampling_priority == Some(SamplingPriority::SamplerKeep)
                {
                    1
                } else {
                    0
                };
            }
            count_sampled as f32 / total as f32
        }

        #[test]
        fn spans_can_be_sampled_config_1() {
            // Case 1, service:nginx,env: => 0.8
            let sample_rate = sampled_test("", "nginx");
            assert!(sample_rate < 0.85 && sample_rate > 0.75);
        }

        #[test]
        fn spans_can_be_sampled_config_2() {
            // Case 2, service:nginx,env:prod => 0.2
            let sample_rate = sampled_test("prod", "nginx");
            assert!(sample_rate < 0.25 && sample_rate > 0.15);
        }
    }
}
