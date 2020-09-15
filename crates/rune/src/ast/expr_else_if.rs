use crate::ast;
use crate::{Parse, ParseError, Parser, Spanned};
use runestick::Span;

/// An else branch of an if expression.
#[derive(Debug, Clone)]
pub struct ExprElseIf {
    /// The `else` token.
    pub else_: ast::Else,
    /// The `if` token.
    pub if_: ast::If,
    /// The condition for the branch.
    pub condition: ast::Condition,
    /// The body of the else statement.
    pub block: Box<ast::ExprBlock>,
}

into_tokens!(ExprElseIf {
    else_,
    if_,
    condition,
    block
});

impl Spanned for ExprElseIf {
    fn span(&self) -> Span {
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
