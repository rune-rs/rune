use crate::ast;
use crate::{Spanned, ToTokens};

/// A try expression `<expr>?`.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub struct ExprTry {
    /// Attributes associated with expression.
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The expression being awaited.
    pub expr: Box<ast::Expr>,
    /// The try operator `?`.
    pub try_token: ast::Try,
}

expr_parse!(ExprTry, "try expression");
