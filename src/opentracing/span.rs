/// SpanContext represents Span state that must propagate to descendant Spans and
/// across process boundaries (e.g., a <trace_id, span_id, sampled> tuple).
pub(crate) trait SpanContext<F>
where
    F: Fn(&str, &str) -> bool,
{
    /// ForeachBaggageItem calls a function for each baggage item in the
    /// context.  If the function returns false, it will not be called
    /// again and ForeachBaggageItem will return.
    fn foreach_baggage_item(f: F);
}
