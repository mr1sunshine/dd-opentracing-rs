use crate::dd::tags::ENVIRONMENT;
use std::collections::HashMap;

#[derive(Default)]
pub(crate) struct SpanData {
    pub span_type: String,
    pub service: String,
    pub resource: String,
    pub name: String,
    pub trace_id: u64,
    pub span_id: u64,
    pub parent_id: u64,
    pub start_id: i64,
    pub duration: i64,
    pub error: i32,
    pub meta: HashMap<String, String>,
    pub metrics: HashMap<String, f64>,
}

impl SpanData {
    pub fn env(&self) -> String {
        match self.meta.get(ENVIRONMENT) {
            Some(env) => env.clone(),
            None => "".to_owned(),
        }
    }
}
