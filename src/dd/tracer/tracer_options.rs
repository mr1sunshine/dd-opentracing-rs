use std::collections::{HashMap, HashSet};

use super::PropagationStyle;

pub struct TracerOptions {
    pub agent_host: String,
    pub agent_port: u16,
    pub service: String,
    pub service_type: String,
    pub environment: String,
    pub sample_rate: f32,
    pub priority_sampling: bool,
    pub sampling_rules: String,
    pub write_perios_ms: u32,
    pub operation_name_override: String,
    pub extract: HashSet<PropagationStyle>,
    pub inject: HashSet<PropagationStyle>,
    pub report_hostname: bool,
    pub analytics_enabled: bool,
    pub analytics_rate: f32,
    pub tags: HashMap<String, String>,
    pub version: String,
    pub agent_url: String,
}
