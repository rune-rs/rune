use crate::ast;
use crate::{Spanned, ToTokens};

/// An assign expression `a = b`.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub struct ExprAssign {
    /// Attributes associated with the assign expression.
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The expression being awaited.
    pub lhs: ast::Expr,
    /// The equals sign `=`.
    pub eq: T![=],
    /// The value.
    pub rhs: ast::Expr,
}

expr_parse!(ExprAssign, "assign expression");
