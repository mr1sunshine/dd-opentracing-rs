use crate::propagation::SpanContext;

pub(crate) trait SpanBuffer {
    fn register_span(context: &SpanContext);
}
