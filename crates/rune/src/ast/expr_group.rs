use crate::ast;
use crate::error::ParseError;
use crate::parser::Parser;
use crate::traits::Parse;
use runestick::Span;

/// A prioritized expression group `(<expr>)`.
#[derive(Debug, Clone)]
pub struct ExprGroup {
    /// The open parenthesis.
    pub open: ast::OpenParen,
    /// The grouped expression.
    pub expr: Box<ast::Expr>,
    /// The close parenthesis.
    pub close: ast::CloseParen,
}

into_tokens!(ExprGroup { open, expr, close });

impl ExprGroup {
    /// Access the span of the expression.
    pub fn span(&self) -> Span {
        self.open.span().join(self.close.span())
    }

    /// Check if expression is empty.
    pub fn produces_nothing(&self) -> bool {
        self.expr.produces_nothing()
    }
}

impl Parse for ExprGroup {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        Ok(Self {
            open: parser.parse()?,
            expr: Box::new(parser.parse()?),
            close: parser.parse()?,
        })
    }
}
