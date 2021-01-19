use crate::dd::span::span_context::SpanContext;

pub(crate) trait SpanBuffer {
    fn register_span(context: &SpanContext);
}
