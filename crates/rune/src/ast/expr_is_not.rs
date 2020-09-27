use crate::ast;
use crate::{Parse, Spanned, ToTokens};

/// An is expression.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Parse, Spanned)]
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
