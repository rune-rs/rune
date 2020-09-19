use crate::ast;
use crate::{Ast, Parse, ParseError, Parser, Spanned};
use runestick::Span;

/// A return statement `<expr>.await`.
#[derive(Debug, Clone, Ast)]
pub struct ExprAwait {
    /// The expression being awaited.
    pub expr: Box<ast::Expr>,
    /// The dot separating the expression.
    pub dot: ast::Dot,
    /// The await token.
    pub await_: ast::Await,
}

impl Spanned for ExprAwait {
    fn span(&self) -> Span {
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
