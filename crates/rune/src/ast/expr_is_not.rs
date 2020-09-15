use crate::ast;
use crate::{Parse, ParseError, Parser, Spanned};
use runestick::Span;

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

into_tokens!(ExprIsNot { lhs, is, not, rhs });

impl ExprIsNot {
    /// If the expression is empty.
    pub fn produces_nothing(&self) -> bool {
        false
    }

    /// Test if the expression is a constant expression.
    pub fn is_const(&self) -> bool {
        self.lhs.is_const() && self.rhs.is_const()
    }
}

impl Spanned for ExprIsNot {
    fn span(&self) -> Span {
        self.lhs.span().join(self.rhs.span())
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
