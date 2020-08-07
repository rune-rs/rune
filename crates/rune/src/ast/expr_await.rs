use crate::ast::{Await, Dot, Expr};
use crate::error::{ParseError, Result};
use crate::parser::Parser;
use crate::traits::Parse;
use stk::unit::Span;

/// A return statement `<expr>.await`.
#[derive(Debug, Clone)]
pub struct ExprAwait {
    /// The expression being awaited.
    pub expr: Box<Expr>,
    /// The dot separating the expression.
    pub dot: Dot,
    /// The await token.
    pub await_: Await,
}

impl ExprAwait {
    /// Access the span of the expression.
    pub fn span(&self) -> Span {
        self.expr.span().join(self.await_.span())
    }
}

impl Parse for ExprAwait {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        Ok(Self {
            expr: Box::new(parser.parse()?),
            dot: parser.parse()?,
            await_: parser.parse()?,
        })
    }
}
