//! Context for a macro.

use crate::TokenStream;
use runestick::{Source, Span};
use std::sync::Arc;

/// Context for a running macro.
pub struct MacroContext {
    source: Arc<Source>,
    /// Temporary recorded default span.
    pub(crate) default_span: Span,
    /// End point of the span.
    pub(crate) end: Span,
}

impl MacroContext {
    /// Construct a new macro context.
    pub fn new(source: Arc<Source>) -> Self {
        Self {
            source,
            default_span: Span::empty(),
            end: Span::empty(),
        }
    }

    /// Access the default span of the context.
    pub fn default_span(&self) -> Span {
        self.default_span
    }

    /// Construct a new token stream.
    pub fn token_stream(&self) -> TokenStream {
        TokenStream::new(Vec::new(), self.end)
    }

    /// Access the current source of the macro context.
    pub fn source(&self) -> &Source {
        &*self.source
    }
}
