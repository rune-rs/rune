use crate::ast;
use crate::{Parse, ParseError, Parser, Spanned};
use runestick::Span;

/// A return statement `return [expr]`.
#[derive(Debug, Clone)]
pub struct ExprReturn {
    /// The return token.
    pub return_: ast::Return,
    /// An optional expression to return.
    pub expr: Option<Box<ast::Expr>>,
}

into_tokens!(ExprReturn { return_, expr });

impl Spanned for ExprReturn {
    fn span(&self) -> Span {
        if let Some(expr) = &self.expr {
            self.return_.span().join(expr.span())
        } else {
            self.return_.span()
        }
    }
}

impl Parse for ExprReturn {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let return_ = parser.parse()?;

        let expr = if parser.peek::<ast::Expr>()? {
            Some(Box::new(parser.parse()?))
        } else {
            None
        };

        Ok(Self { return_, expr })
    }
}
