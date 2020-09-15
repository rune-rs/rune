use crate::ast;
use crate::Spanned;
use runestick::Span;

/// A tuple pattern.
#[derive(Debug, Clone)]
pub struct PatPath {
    /// The path, if the tuple is typed.
    pub path: ast::Path,
}

into_tokens!(PatPath { path });

impl Spanned for PatPath {
    fn span(&self) -> Span {
        self.path.span()
    }
}
