// Original tracing types (OTEL structures)
mod shapes;
// New tracing utilities
mod span_builder;
mod resource_span_builder;
mod constants;

#[cfg(feature = "trace-collection")]
mod collector;

#[cfg(all(test, feature = "trace-collection"))]
mod tests;

// Re-export original types
pub use shapes::{
    Span, Event, Traceparent, TraceparentNewError,
    ResourceSpan, Resource, ScopeSpan, Scope, Attribute, AttributeValue,
};

// Re-export new utilities
pub use span_builder::{SpanBuilder, SpanKind};
pub use resource_span_builder::ResourceSpanBuilder;
pub use constants::*;

#[cfg(feature = "trace-collection")]
pub use collector::{TraceCollector, parse_traceparent};
