use eyre::Result;

use super::{SpanContext, Tracer};

#[derive(Clone)]
pub(crate) enum SpanReferenceType {
    /// ChildOfRef refers to a parent Span that caused *and* somehow depends
    /// upon the new child Span. Often (but not always), the parent Span cannot
    /// finish until the child Span does.
    ///
    /// An timing diagram for a ChildOfRef that's blocked on the new Span:
    ///
    ///     [-Parent Span---------]
    ///          [-Child Span----]
    ///
    /// See http://opentracing.io/spec/
    ///
    /// See opentracing.ChildOf()
    ChildOfRef,
    /// FollowsFromRef refers to a parent Span that does not depend in any way
    /// on the result of the new child Span. For instance, one might use
    /// FollowsFromRefs to describe pipeline stages separated by queues,
    /// or a fire-and-forget cache insert at the tail end of a web request.
    ///
    /// A FollowsFromRef Span is part of the same logical trace as the new Span:
    /// i.e., the new Span is somehow caused by the work of its FollowsFromRef.
    ///
    /// All of the following could be valid timing diagrams for children that
    /// "FollowFrom" a parent.
    ///
    ///     [-Parent Span-]  [-Child Span-]
    ///
    ///
    ///     [-Parent Span--]
    ///      [-Child Span-]
    ///
    ///
    ///     [-Parent Span-]
    ///                 [-Child Span-]
    ///
    /// See http://opentracing.io/spec/
    ///
    /// See opentracing.FollowsFrom()
    FollowsFromRef,
}

pub(crate) enum PropagationError {
    /// `InvalidSpanContext` occurs when Tracer::Inject() is asked to operate
    /// on a SpanContext which it is not prepared to handle (for example, since it
    /// was created by a different tracer implementation).
    InvalidSpanContext,
    /// `InvalidCarrier` occurs when Tracer::Inject() or Tracer::Extract()
    /// implementations expect a different type of `carrier` than they are given.
    InvalidCarrier,
    /// `SpanContextCorrupted` occurs when the `carrier` passed to
    /// Tracer::Extract() is of the expected type but is corrupted.
    SpanContextCorrupted,
    /// `KeyNotFound` occurs when TextMapReader::LookupKey fails to find
    /// an entry for the provided key.
    KeyNotFound,
    /// `LookupKeyNotSupported` occurs when TextMapReader::LookupKey is
    /// not supported for the provided key.
    LookupKeyNotSupported,
}

/// TextMapReader is the Extract() carrier for the TextMap builtin format. With
/// it, the caller can decode a SpanContext from entries in a propagated map of
/// Unicode strings.
///
/// See the HTTPHeaders examples.
pub(crate) trait TextMapReader {
    /// LookupKey returns the value for the specified `key` if available. If no
    /// such key is present, it returns `PropagationError::KeyNotFound`.
    ///
    /// TextMapReaders are not required to implement this method. If not supported,
    /// the function returns `PropagationError::LookupKeyNotSupported`.
    ///
    /// Tracers may use this as an alternative to `ForeachKey` as a faster way to
    /// extract span context.
    fn lookup_key(&self, key: &str) -> Result<String, PropagationError>;

    /// ForeachKey returns TextMap contents via repeated calls to the `f`
    /// function. If any call to `f` returns an error, ForeachKey terminates and
    /// returns that error.
    ///
    /// NOTE: The backing store for the TextMapReader may contain data unrelated
    /// to SpanContext. As such, Inject() and Extract() implementations that
    /// call the TextMapWriter and TextMapReader interfaces must agree on a
    /// prefix or other convention to distinguish their own key:value pairs.
    ///
    /// The "foreach" callback pattern reduces unnecessary copying in some cases
    /// and also allows implementations to hold locks while the map is read.
    fn foreach_key<F>(&self, f: F) -> Result<()>
    where
        F: Fn(&str, &str) -> Result<()>,
        Self: Sized;
}

/// TextMapWriter is the Inject() carrier for the TextMap builtin format. With
/// it, the caller can encode a SpanContext for propagation as entries in a map
/// of unicode strings.
///
/// See the HTTPHeaders examples.
pub(crate) trait TextMapWriter {
    /// Set a key:value pair to the carrier. Multiple calls to Set() for the
    /// same key leads to undefined behavior.
    ///
    /// NOTE: The backing store for the TextMapWriter may contain data unrelated
    /// to SpanContext. As such, Inject() and Extract() implementations that
    /// call the TextMapWriter and TextMapReader interfaces must agree on a
    /// prefix or other convention to distinguish their own key:value pairs.
    fn set(&mut self, key: &str, value: &str) -> Result<()>;
}

/// HTTPHeadersReader is the Extract() carrier for the HttpHeaders builtin
/// format. With it, the caller can decode a SpanContext from entries in HTTP
/// request headers.
pub(crate) trait HTTPHeadersReader: TextMapReader {}

/// HTTPHeadersWriter is the Inject() carrier for the TextMap builtin format.
/// With it, the caller can encode a SpanContext for propagation as entries in
/// http request headers
pub(crate) trait HTTPHeadersWriter: TextMapWriter {}

/// CustomCarrierReader is the Extract() carrier for a custom format. With it,
/// the caller can decode a SpanContext from entries in a custom protocol.
pub(crate) trait CustomCarrierReader {
    /// Extract is expected to specialize on the tracer implementation so as to
    /// most efficiently decode its context.
    fn extract(&self, tracer: &dyn Tracer) -> Result<Box<dyn SpanContext>>;
}

/// CustomCarrierWriter is the Inject() carrier for a custom format.  With it,
/// the caller can encode a SpanContext for propagation as entries in a custom
/// protocol.
pub(crate) trait CustomCarrierWriter {
    /// Inject is expected to specialize on the tracer implementation so as to most
    /// efficiently encode its context.
    fn inject(tracer: &dyn Tracer, sc: &dyn SpanContext) -> Result<()>;
}
