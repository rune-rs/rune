use crate::ast::{Condition, Else, ExprBlock, If};
use crate::error::ParseError;
use crate::parser::Parser;
use crate::traits::Parse;
use runestick::unit::Span;

/// An else branch of an if expression.
#[derive(Debug, Clone)]
pub struct ExprElseIf {
    /// The `else` token.
    pub else_: Else,
    /// The `if` token.
    pub if_: If,
    /// The condition for the branch.
    pub condition: Condition,
    /// The body of the else statement.
    pub block: Box<ExprBlock>,
}

impl ExprElseIf {
    /// Access the span for the expression.
    pub fn span(&self) -> Span {
        self.else_.span().join(self.block.span())
    }
}

impl Parse for ExprElseIf {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        Ok(Self {
            else_: parser.parse()?,
            if_: parser.parse()?,
            condition: parser.parse()?,
            block: Box::new(parser.parse()?),
        })
    }
}
