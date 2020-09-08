use crate::ast::{CloseBracket, Expr, OpenBracket};
use runestick::Span;

/// An index get operation `<target>[<index>]`.
#[derive(Debug, Clone)]
pub struct ExprIndexGet {
    /// The target of the index set.
    pub target: Box<Expr>,
    /// The opening bracket.
    pub open: OpenBracket,
    /// The indexing expression.
    pub index: Box<Expr>,
    /// The closening bracket.
    pub close: CloseBracket,
}

into_tokens!(ExprIndexGet {
    target,
    open,
    index,
    close
});

impl ExprIndexGet {
    /// Access the span of the expression.
    pub fn span(&self) -> Span {
        self.target.span().join(self.close.span())
    }
}
