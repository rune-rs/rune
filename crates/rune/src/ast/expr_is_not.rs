use crate::ast;
use crate::error::ParseError;
use crate::parser::Parser;
use crate::traits::Parse;
use runestick::unit::Span;

/// An is expression.
#[derive(Debug, Clone)]
pub struct ExprIsNot {
    /// The left-hand side of a is operation.
    pub lhs: Box<ast::Expr>,
    /// The `is` keyword.
    pub is: ast::Is,
    /// The `not` keyword.
    pub not: ast::Not,
    /// The right-hand side of a is operation.
    pub rhs: Box<ast::Expr>,
}

impl ExprIsNot {
    /// If the expression is empty.
    pub fn produces_nothing(&self) -> bool {
        false
    }

    /// Access the span of the expression.
    pub fn span(&self) -> Span {
        self.lhs.span().join(self.rhs.span())
    }

    /// Test if the expression is a constant expression.
    pub fn is_const(&self) -> bool {
        self.lhs.is_const() && self.rhs.is_const()
    }
}

impl Parse for ExprIsNot {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        Ok(Self {
            lhs: Box::new(parser.parse()?),
            is: parser.parse()?,
            not: parser.parse()?,
            rhs: Box::new(parser.parse()?),
        })
    }
}
