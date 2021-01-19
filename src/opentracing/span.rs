use std::time::{Instant, SystemTime};

use eyre::Result;
use serde_json::Value;

use super::Tracer;

/// SpanContext represents Span state that must propagate to descendant Spans and
/// across process boundaries (e.g., a <trace_id, span_id, sampled> tuple).
pub(crate) trait SpanContext {
    /// ForeachBaggageItem calls a function for each baggage item in the
    /// context.  If the function returns false, it will not be called
    /// again and ForeachBaggageItem will return.
    fn foreach_baggage_item<F>(&self, f: F) -> Result<()>
    where
        F: Fn(&str, &str) -> bool,
        Self: Sized;
}

pub(crate) struct LogRecord {
    pub timestamp: SystemTime,
    pub fields: Vec<(String, Value)>,
}

/// FinishOptions allows Span.Finish callers to override the finish
/// timestamp.
pub(crate) struct FinishSpanOptions {
    pub finish_steady_timestamp: Instant,

    /// log_records allows the caller to specify the contents of many Log() calls
    /// with a single vector. May be empty.
    ///
    /// None of the LogRecord.timestamp values may be SystemTime() (i.e., they must
    /// be set explicitly). Also, they must be >= the Span's start system timestamp
    /// and <= the finish_steady_timestamp converted to system timestamp
    /// (or SystemTime::now() if finish_steady_timestamp is default-constructed).
    /// Otherwise the behavior of FinishWithOptions() is unspecified.
    pub log_records: Vec<LogRecord>,
}

/// FinishSpanOption instances (zero or more) may be passed to Span.Finish.
pub(crate) trait FinishSpanOption {
    fn apply(&mut self, options: &mut FinishSpanOptions);
}

/// Span represents an active, un-finished span in the OpenTracing system.
///
/// Spans are created by the Tracer interface.
pub(crate) trait Span {
    /// Sets the end timestamp and finalizes Span state.
    ///
    /// If Finish is called a second time, it is guaranteed to do nothing.
    fn finish(&mut self, option_list: Vec<&mut Box<dyn FinishSpanOption>>) {
        let mut options = FinishSpanOptions {
            finish_steady_timestamp: Instant::now(),
            log_records: Vec::new(),
        };

        for option in option_list {
            option.apply(&mut options);
        }

        self.finish_with_options(&options);
    }

    fn finish_with_options(&mut self, finish_span_options: &FinishSpanOptions);

    /// Sets or changes the operation name.
    ///
    /// If SetOperationName is called after Finish it leaves the Span in a valid
    /// state, but its behavior is unspecified.
    fn set_operation_name(&mut self, operation_name: &str);

    /// Adds a tag to the span.
    ///
    /// If there is a pre-existing tag set for `key`, it is overwritten.
    ///
    /// Tag values can be numeric types, strings, or bools. The behavior of
    /// other tag value types is undefined at the OpenTracing level. If a
    /// tracing system does not know how to handle a particular value type, it
    /// may ignore the tag, but shall not panic.
    ///
    /// If SetTag is called after Finish it leaves the Span in a valid state, but
    /// its behavior is unspecified.
    fn set_tag(&mut self, key: &str, value: &Value);

    /// SetBaggageItem sets a key:value pair on this Span and its SpanContext
    /// that also propagates to descendants of this Span.
    ///
    /// SetBaggageItem() enables powerful functionality given a full-stack
    /// opentracing integration (e.g., arbitrary application data from a mobile
    /// app can make it, transparently, all the way into the depths of a storage
    /// system), and with it some powerful costs: use this feature with care.
    ///
    /// IMPORTANT NOTE #1: SetBaggageItem() will only propagate baggage items to
    /// *future* causal descendants of the associated Span.
    ///
    /// IMPORTANT NOTE #2: Use this thoughtfully and with care. Every key and
    /// value is copied into every local *and remote* child of the associated
    /// Span, and that can add up to a lot of network and cpu overhead.
    ///
    /// If SetBaggageItem is called after Finish it leaves the Span in a valid
    /// state, but its behavior is unspecified.
    fn set_baggage_item(&mut self, restricted_key: &str, value: &str);

    /// Gets the value for a baggage item given its key. Returns the empty string
    /// if the value isn't found in this Span.
    fn baggage_item(&self, restricted_key: &str) -> String;

    fn log(&mut self, fields: &[(String, Value)]);

    /// context() yields the SpanContext for this Span. Note that the return
    /// value of context() is still valid after a call to Span.Finish(), as is
    /// a call to Span.context() after a call to Span.Finish().
    fn context(&self) -> &dyn SpanContext;

    /// Provides access to the Tracer that created this Span.
    fn tracer(&self) -> &Tracer;
}
