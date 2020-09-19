use crate::ast;
use crate::{Ast, Parse, ParseError, Parser, Spanned};
use runestick::Span;

/// A return statement `break [expr]`.
#[derive(Debug, Clone, Ast)]
pub struct ExprYield {
    /// The return token.
    pub yield_: ast::Yield,
    /// An optional expression to yield.
    pub expr: Option<Box<ast::Expr>>,
}

impl Spanned for ExprYield {
    fn span(&self) -> Span {
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
