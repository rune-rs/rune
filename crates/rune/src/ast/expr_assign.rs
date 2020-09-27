use crate::ast;
use crate::{Spanned, ToTokens};

/// An assign expression `a = b`.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub struct ExprAssign {
    /// Attributes associated with the assign expression.
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The expression being awaited.
    pub lhs: Box<ast::Expr>,
    /// The equals sign `=`.
    pub eq: ast::Eq,
    /// The value.
    pub rhs: Box<ast::Expr>,
}
