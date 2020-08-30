use crate::ast::{Comma, Expr, Parenthesized};
use runestick::unit::Span;

/// A function call `<name>(<args>)`.
#[derive(Debug, Clone)]
pub struct CallFn {
    /// The name of the function being called.
    pub expr: Box<Expr>,
    /// The arguments of the function call.
    pub args: Parenthesized<Expr, Comma>,
}

impl CallFn {
    /// Access the span of expression.
    pub fn span(&self) -> Span {
        self.expr.span().join(self.args.span())
    }
}
