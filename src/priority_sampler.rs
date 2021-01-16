use crate::sampling_priority::SamplingPriority;
use eyre::{eyre, Result};
use serde_json::Value;
use std::{collections::HashMap, sync::Mutex};

const CONSTANT_RATE_HASH_FACTOR: u64 = 1111111111111111111;
const PRIORITY_SAMPLER_DEFAULT_RATE_KEY: &str = "service:,env:";
const MAX_TRACE_ID_DOUBLE: f64 = std::f64::MAX as f64;

#[derive(Default)]
pub struct SampleResult {
    pub rule_rate: f32,
    pub limiter_rate: f32,
    pub priority_rate: f32,
    pub sampling_priority: Option<SamplingPriority>,
}

#[derive(Default, Clone)]
pub struct SamplingRate {
    pub rate: f32,
    pub max_hash: u64,
}

struct PrioritySamplerData {
    pub agent_sampling_rates: HashMap<String, SamplingRate>,
    pub default_sampling_rate: SamplingRate,
}

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

        let sampling_priority = if trace_id * CONSTANT_RATE_HASH_FACTOR >= applied_rate.max_hash {
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

    fn max_id_from_sample_rate(rate: f64) -> u64 {
        if rate == 1.0 {
            std::u64::MAX
        } else if rate > 0.0 {
            (rate * MAX_TRACE_ID_DOUBLE) as u64
        } else {
            0
        }
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
                max_hash: PrioritySampler::max_id_from_sample_rate(rate),
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
