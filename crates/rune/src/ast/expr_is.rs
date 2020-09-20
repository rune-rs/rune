use crate::ast;
use crate::{Parse, Spanned, ToTokens};

/// An is expression.
#[derive(Debug, Clone, ToTokens, Parse, Spanned)]
pub struct ExprIs {
    /// The left-hand side of a is operation.
    pub lhs: Box<ast::Expr>,
    /// The `is` keyword.
    pub is: ast::Is,
    /// The right-hand side of a is operation.
    pub rhs: Box<ast::Expr>,
}

impl ExprIs {
    /// If the expression is empty.
    pub fn produces_nothing(&self) -> bool {
        false
    }

    /// Test if the expression is a constant expression.
    pub fn is_const(&self) -> bool {
        self.lhs.is_const() && self.rhs.is_const()
    }
}
