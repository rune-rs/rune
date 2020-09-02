use crate::ast;
use crate::error::{ParseError, Result};
use crate::parser::Parser;
use crate::traits::Parse;
use runestick::unit::Span;

/// A return statement `break [expr]`.
#[derive(Debug, Clone)]
pub struct ExprYield {
    /// The return token.
    pub yield_: ast::Yield,
    /// An optional expression to yield.
    pub expr: Option<Box<ast::Expr>>,
}

impl ExprYield {
    /// Access the span of the expression.
    pub fn span(&self) -> Span {
        if let Some(expr) = &self.expr {
            self.yield_.span().join(expr.span())
        } else {
            self.yield_.span()
        }
    }
}

impl Parse for ExprYield {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        Ok(Self {
            yield_: parser.parse()?,
            expr: parser.parse()?,
        })
    }
}
