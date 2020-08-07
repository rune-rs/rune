use crate::ast::{Comma, Expr, Parenthesized, Path};
use runestick::unit::Span;

/// A function call `<name>(<args>)`.
#[derive(Debug, Clone)]
pub struct CallFn {
    /// The name of the function being called.
    pub name: Path,
    /// The arguments of the function call.
    pub args: Parenthesized<Expr, Comma>,
}

impl CallFn {
    /// Access the span of expression.
    pub fn span(&self) -> Span {
        self.name.span().join(self.args.span())
    }
}
