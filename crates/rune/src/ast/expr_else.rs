use crate::ast::{Else, ExprBlock};
use crate::{Parse, ParseError, Parser, Spanned};
use runestick::Span;

/// An else branch of an if expression.
#[derive(Debug, Clone)]
pub struct ExprElse {
    /// The `else` token.
    pub else_: Else,
    /// The body of the else statement.
    pub block: Box<ExprBlock>,
}

into_tokens!(ExprElse { else_, block });

impl Spanned for ExprElse {
    fn span(&self) -> Span {
        self.else_.span().join(self.block.span())
    }
}

impl Parse for ExprElse {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        Ok(ExprElse {
            else_: parser.parse()?,
            block: Box::new(parser.parse()?),
        })
    }
}
