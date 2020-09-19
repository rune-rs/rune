use crate::ast;
use crate::{Ast, Spanned};
use runestick::Span;

/// A tuple pattern.
#[derive(Debug, Clone, Ast)]
pub struct PatPath {
    /// The path, if the tuple is typed.
    pub path: ast::Path,
}

impl Spanned for PatPath {
    fn span(&self) -> Span {
        self.path.span()
    }
}
