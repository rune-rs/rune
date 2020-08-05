use crate::ast::{CloseBracket, Eq, Expr, OpenBracket};
use st::unit::Span;

/// An index set operation `<target>[<index>] = <value>`.
#[derive(Debug, Clone)]
pub struct ExprIndexSet {
    /// The target of the index set.
    pub target: Box<Expr>,
    /// The opening bracket.
    pub open: OpenBracket,
    /// The indexing expression.
    pub index: Box<Expr>,
    /// The closening bracket.
    pub close: CloseBracket,
    /// The equals sign.
    pub eq: Eq,
    /// The value expression we are assigning.
    pub value: Box<Expr>,
}

impl ExprIndexSet {
    /// Access the span of the expression.
    pub fn span(&self) -> Span {
        self.target.span().join(self.value.span())
    }
}
