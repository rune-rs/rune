use crate::ast::{Else, ExprBlock};
use crate::error::ParseError;
use crate::parser::Parser;
use crate::traits::Parse;
use runestick::unit::Span;

/// An else branch of an if expression.
#[derive(Debug, Clone)]
pub struct ExprElse {
    /// The `else` token.
    pub else_: Else,
    /// The body of the else statement.
    pub block: Box<ExprBlock>,
}

impl ExprElse {
    /// Access the span for the expression.
    pub fn span(&self) -> Span {
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
