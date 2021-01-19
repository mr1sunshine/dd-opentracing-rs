use crate::{dd::sample::SamplingPriority, opentracing};
use eyre::{eyre, Result};
use std::{collections::HashMap, sync::Mutex};

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

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn trace_id(&self) -> u64 {
        self.trace_id
    }

    pub fn propagated_sampling_priority(&self) -> &Option<SamplingPriority> {
        &self.propagated_sampling_priority
    }

    pub fn origin(&self) -> &str {
        &self.origin
    }

    pub fn set_baggage_item(&mut self, key: &str, value: &str) -> Result<()> {
        let mut data = self
            .baggage
            .lock()
            .map_err(|_| eyre!("mutex lock failed"))?;

        data.insert(String::from(key), String::from(value));

        Ok(())
    }

    pub fn baggage_item(&self, key: &str) -> Result<Option<String>> {
        let data = self
            .baggage
            .lock()
            .map_err(|_| eyre!("mutex lock failed"))?;

        Ok(match data.get(key) {
            Some(str) => Some(str.clone()),
            None => None,
        })
    }

    pub fn with_id(&self, id: u64) -> Result<SpanContext> {
        let data = self
            .baggage
            .lock()
            .map_err(|_| eyre!("mutex lock failed"))?;

        let baggage = data.clone();
        let mut context = SpanContext::new(id, self.trace_id, &self.origin, baggage);
        context.propagated_sampling_priority = self.propagated_sampling_priority.clone();

        Ok(context)
    }
}

impl<F> opentracing::SpanContext<F> for SpanContext
where
    F: Fn(&str, &str) -> bool,
{
    fn foreach_baggage_item(&self, f: F) -> Result<()> {
        let data = self
            .baggage
            .lock()
            .map_err(|_| eyre!("mutex lock failed"))?;

        for (key, value) in data.iter() {
            if !f(key, value) {
                return Ok(());
            }
        }

        Ok(())
    }
}
