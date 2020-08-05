use crate::ast::{Comma, Dot, Expr, Ident, Parenthesized};
use st::unit::Span;

/// An instance function call `<instance>.<name>(<args>)`.
#[derive(Debug, Clone)]
pub struct CallInstanceFn {
    /// The instance being called.
    pub instance: Box<Expr>,
    /// The parsed dot separator.
    pub dot: Dot,
    /// The name of the function being called.
    pub name: Ident,
    /// The arguments of the function call.
    pub args: Parenthesized<Expr, Comma>,
}

impl CallInstanceFn {
    /// Access the span of the expression.
    pub fn span(&self) -> Span {
        self.instance.span().join(self.args.span())
    }
}
