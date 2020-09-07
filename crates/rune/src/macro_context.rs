//! Context for a macro.

use crate::ast;
use crate::{Storage, TokenStream};
use runestick::{Source, Span};
use std::sync::Arc;

/// Context for a running macro.
pub struct MacroContext {
    /// The current source.
    source: Arc<Source>,
    /// Temporary recorded default span.
    pub(crate) default_span: Span,
    /// End point of the span.
    pub(crate) end: Span,
    /// Storage used in macro context.
    pub(crate) storage: Storage,
}

impl MacroContext {
    /// Construct a new macro context.
    pub fn new(storage: Storage, source: Arc<Source>) -> Self {
        Self {
            source,
            default_span: Span::empty(),
            end: Span::empty(),
            storage,
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

    /// Access storage for the macro system.
    pub fn storage(&self) -> &Storage {
        &self.storage
    }

    /// Access the current source of the macro context.
    pub fn source(&self) -> &Source {
        &*self.source
    }

    /// Generate an identifier.
    pub fn ident(&self, ident: &str) -> ast::Token {
        let id = self.storage.insert_ident(ident);

        ast::Token {
            span: self.default_span,
            kind: ast::Kind::Ident(ast::IdentKind::Synthetic(id)),
        }
    }
}
