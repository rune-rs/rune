use crate::ast;
use crate::{Spanned, ToTokens};

/// A try expression `<expr>?`.
#[derive(Debug, Clone, ToTokens, Spanned)]
pub struct ExprTry {
    /// The expression being awaited.
    pub expr: Box<ast::Expr>,
    /// The try operator.
    pub try_: ast::Try,
}
