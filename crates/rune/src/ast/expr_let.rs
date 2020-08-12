use crate::ast::{Eq, Expr, Let, Pat};
use crate::error::ParseError;
use crate::parser::Parser;
use crate::traits::Parse;
use runestick::unit::Span;

/// A let expression `let <name> = <expr>;`
#[derive(Debug, Clone)]
pub struct ExprLet {
    /// The `let` keyword.
    pub let_: Let,
    /// The name of the binding.
    pub pat: Pat,
    /// The equality keyword.
    pub eq: Eq,
    /// The expression the binding is assigned to.
    pub expr: Box<Expr>,
}

impl ExprLet {
    /// Access the span of the expression.
    pub fn span(&self) -> Span {
        self.let_.token.span.join(self.expr.span())
    }

    /// Parse a let expression without eager bracing.
    pub fn parse_without_eager_brace(parser: &mut Parser) -> Result<Self, ParseError> {
        Ok(Self {
            let_: parser.parse()?,
            pat: parser.parse()?,
            eq: parser.parse()?,
            expr: Box::new(Expr::parse_without_eager_brace(parser)?),
        })
    }
}

impl Parse for ExprLet {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        Ok(Self {
            let_: parser.parse()?,
            pat: parser.parse()?,
            eq: parser.parse()?,
            expr: Box::new(parser.parse()?),
        })
    }
}
