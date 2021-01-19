use super::{Span, SpanContext, SpanReferenceType, TextMapReader, TextMapWriter};
use eyre::Result;
use serde_json::Value;
use std::{
    rc::Rc,
    time::{Instant, SystemTime},
};

/// StartSpanOptions allows Tracer.StartSpan() callers a mechanism to override
/// the start timestamp, specify Span References, and make a single Tag or
/// multiple Tags available at Span start time.
///
/// StartSpan() callers should look at the StartSpanOption interface and
/// implementations available in this library.
pub(crate) struct StartSpanOptions {
    /// start_system_timestamp and start_steady_timestamp override the Span's start
    /// time, or implicitly become std::chrono::system_clock::now() and
    /// std::chrono::steady_clock::now() if both are equal to the epoch (default
    /// behavior).
    ///
    /// If one of the timestamps is set but not the other, the set timestamp is
    /// used to estimate the corresponding timestamp of the other.
    pub start_system_time: SystemTime,
    pub start_steady_time: Instant,
    /// Zero or more causal references to other Spans (via their SpanContext).
    /// If empty, start a "root" Span (i.e., start a new trace).
    ///
    /// Any nullptrs provided will be ignored.
    pub references: Vec<(SpanReferenceType, Rc<dyn SpanContext>)>,
    /// Zero or more tags to apply to the newly created span.
    pub tags: Vec<(String, Value)>,
}

impl Default for StartSpanOptions {
    fn default() -> StartSpanOptions {
        StartSpanOptions {
            start_system_time: SystemTime::now(),
            start_steady_time: Instant::now(),
            references: Vec::new(),
            tags: Vec::new(),
        }
    }
}

/// StartSpanOption instances (zero or more) may be passed to Tracer.StartSpan.
pub(crate) trait StartSpanOption {
    fn apply(&mut self, options: &mut StartSpanOptions);
}

/// Tracer is a simple, thin interface for Span creation and SpanContext
/// propagation.
pub(crate) trait Tracer {
    /// Create, start, and return a new Span with the given `operationName` and
    /// incorporate the given StartSpanOption `option_list`.
    ///
    /// A Span with no SpanReference options (e.g., opentracing::ChildOf() or
    /// opentracing::FollowsFrom()) becomes the root of its own trace.
    fn start_span(
        &self,
        operation_name: &str,
        option_list: Vec<Box<dyn StartSpanOption>>,
    ) -> Box<dyn Span + '_> {
        let mut options = StartSpanOptions::default();
        for mut option in option_list {
            option.apply(&mut options);
        }

        self.start_span_with_options(operation_name, &options)
    }

    fn start_span_with_options(
        &self,
        operation_name: &str,
        options: &StartSpanOptions,
    ) -> Box<dyn Span + '_>;

    fn inject(&mut self, sc: &dyn SpanContext, writer: &dyn TextMapWriter) -> Result<()>;
    fn extract(&self, reader: &dyn TextMapReader) -> Result<Box<dyn SpanContext>>;

    fn close(&mut self);
}

// static mut GLOBAL_TRACER: Rc<dyn Tracer> = Rc::new();

// pub(crate) fn init_global(tracer: Rc<dyn Tracer>) {
//     static
// }
pub(crate) struct StartTimestamp {
    system_when: SystemTime,
    steady_when: Instant,
}

impl StartTimestamp {
    pub fn new(system_when: SystemTime, steady_when: Instant) -> Self {
        Self {
            system_when,
            steady_when,
        }
    }
}

impl StartSpanOption for StartTimestamp {
    fn apply(&mut self, options: &mut StartSpanOptions) {
        options.start_system_time = self.system_when;
        options.start_steady_time = self.steady_when;
    }
}

pub(crate) struct SpanReference {
    span_ref_type: SpanReferenceType,
    referenced: Rc<dyn SpanContext>,
}

impl SpanReference {
    pub fn new(span_ref_type: SpanReferenceType, referenced: Rc<dyn SpanContext>) -> Self {
        Self {
            span_ref_type,
            referenced,
        }
    }
}

impl StartSpanOption for SpanReference {
    fn apply(&mut self, options: &mut StartSpanOptions) {
        options
            .references
            .push((self.span_ref_type.clone(), self.referenced.clone()));
    }
}

pub(crate) fn child_of(sc: Rc<dyn SpanContext>) -> SpanReference {
    SpanReference::new(SpanReferenceType::ChildOfRef, sc)
}

pub(crate) fn follows_from(sc: Rc<dyn SpanContext>) -> SpanReference {
    SpanReference::new(SpanReferenceType::FollowsFromRef, sc)
}

pub(crate) struct SetTag {
    key: String,
    value: Value,
}

impl SetTag {
    pub fn new(key: &str, value: &Value) -> Self {
        Self {
            key: String::from(key),
            value: value.clone(),
        }
    }
}

impl StartSpanOption for SetTag {
    fn apply(&mut self, options: &mut StartSpanOptions) {
        options.tags.push((self.key.clone(), self.value.clone()));
    }
}
