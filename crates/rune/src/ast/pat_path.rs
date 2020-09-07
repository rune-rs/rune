use crate::ast;
use runestick::Span;

/// A tuple pattern.
#[derive(Debug, Clone)]
pub struct PatPath {
    /// The path, if the tuple is typed.
    pub path: ast::Path,
}

into_tokens!(PatPath { path });

impl PatPath {
    /// Get the span of the pattern.
    pub fn span(&self) -> Span {
        self.path.span()
    }
}
