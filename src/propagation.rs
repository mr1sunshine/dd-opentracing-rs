use std::{collections::HashMap, sync::Mutex};

use crate::{opentracing, sampling_priority::SamplingPriority};

pub(crate) struct SpanContext {
    nginx_opentracing_compatibility_hack: bool,
    propagated_sampling_priority: Option<SamplingPriority>,
    id: u64,
    trace_id: u64,
    origin: String,

    baggage: Mutex<HashMap<String, String>>,
}

impl SpanContext {
    pub fn new(
        id: u64,
        trace_id: u64,
        origin: &str,
        baggage: HashMap<String, String>,
    ) -> SpanContext {
        Self {
            nginx_opentracing_compatibility_hack: false,
            propagated_sampling_priority: None,
            id,
            trace_id,
            origin: String::from(origin),
            baggage: Mutex::new(baggage),
        }
    }

    pub fn new_nginx_opentracing_compatibility_hack(
        id: u64,
        trace_id: u64,
        baggage: HashMap<String, String>,
    ) -> SpanContext {
        Self {
            nginx_opentracing_compatibility_hack: true,
            ..SpanContext::new(id, trace_id, "", baggage)
        }
    }
}

impl<F> opentracing::SpanContext<F> for SpanContext
where
    F: Fn(&str, &str) -> bool,
{
    fn foreach_baggage_item(f: F) {
        todo!()
    }
}
