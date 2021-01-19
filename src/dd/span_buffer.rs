use crate::dd::propagation::SpanContext;

pub(crate) trait SpanBuffer {
    fn register_span(context: &SpanContext);
}
