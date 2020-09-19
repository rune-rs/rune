use crate::ast;
use crate::{Ast, Spanned};

/// A try expression `<expr>?`.
#[derive(Debug, Clone, Ast, Spanned)]
pub struct ExprTry {
    /// The expression being awaited.
    pub expr: Box<ast::Expr>,
    /// The try operator.
    pub try_: ast::Try,
}
