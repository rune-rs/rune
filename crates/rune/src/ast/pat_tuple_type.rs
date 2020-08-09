use crate::ast;
use runestick::unit::Span;

/// A tuple type pattern.
#[derive(Debug, Clone)]
pub struct PatTupleType {
    /// The identifier of the type to match.
    pub path: ast::Path,
    /// The tuple pattern to match.
    pub pat_tuple: ast::PatTuple,
}

impl PatTupleType {
    /// Get the span of the pattern.
    pub fn span(&self) -> Span {
        self.path.span().join(self.pat_tuple.span())
    }
}
