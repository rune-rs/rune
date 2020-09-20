use crate::ast;
use crate::{Parse, ParseError, Parser, Spanned, ToTokens};

/// A return statement `<expr>.await`.
#[derive(Debug, Clone, ToTokens, Spanned)]
pub struct ExprAwait {
    /// The expression being awaited.
    pub expr: Box<ast::Expr>,
    /// The dot separating the expression.
    pub dot: ast::Dot,
    /// The await token.
    pub await_: ast::Await,
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
