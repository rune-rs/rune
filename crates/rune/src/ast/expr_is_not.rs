use crate::ast;
use crate::{Parse, Spanned, ToTokens};

/// An is expression.
#[derive(Debug, Clone, ToTokens, Parse, Spanned)]
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

    /// Test if the expression is a constant expression.
    pub fn is_const(&self) -> bool {
        self.lhs.is_const() && self.rhs.is_const()
    }
}
