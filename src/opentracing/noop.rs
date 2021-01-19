use std::rc::Rc;

use super::{Span, SpanContext, Tracer};
use eyre::Result;

pub(crate) struct NoopSpanContext {}

impl SpanContext for NoopSpanContext {
    fn foreach_baggage_item<F>(&self, f: F) -> Result<()>
    where
        F: Fn(&str, &str) -> bool,
    {
        Ok(())
    }
}

pub(crate) struct NoopSpan<'a> {
    tracer: &'a dyn Tracer,
    span_context: NoopSpanContext,
}

impl<'a> NoopSpan<'a> {
    pub fn new(tracer: &'a dyn Tracer) -> NoopSpan {
        Self {
            tracer,
            span_context: NoopSpanContext {},
        }
    }
}

impl<'a> Span for NoopSpan<'a> {
    fn finish_with_options(&mut self, _finish_span_options: &super::FinishSpanOptions) {}

    fn set_operation_name(&mut self, _operation_name: &str) {}

    fn set_tag(&mut self, _key: &str, _value: &serde_json::Value) {}

    fn set_baggage_item(&mut self, _restricted_key: &str, _value: &str) {}

    fn baggage_item(&self, _restricted_key: &str) -> String {
        String::new()
    }

    fn log(&mut self, _fields: &[(String, serde_json::Value)]) {}

    fn context(&self) -> &dyn SpanContext {
        &self.span_context
    }

    fn tracer(&self) -> &dyn Tracer {
        self.tracer
    }
}

pub(crate) struct NoopTracer {}

impl Tracer for NoopTracer {
    fn start_span_with_options(
        &self,
        _operation_name: &str,
        _options: &super::StartSpanOptions,
    ) -> Box<dyn Span + '_> {
        Box::new(NoopSpan::new(self))
    }

    fn inject(&mut self, _sc: &dyn SpanContext, _writer: &dyn super::TextMapWriter) -> Result<()> {
        Ok(())
    }

    fn extract(&self, _reader: &dyn super::TextMapReader) -> Result<Box<dyn SpanContext>> {
        Ok(Box::new(NoopSpanContext {}))
    }

    fn close(&mut self) {}
}
