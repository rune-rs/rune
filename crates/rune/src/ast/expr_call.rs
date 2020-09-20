use crate::ast;
use crate::{Spanned, ToTokens};

/// A function call `<expr>(<args>)`.
#[derive(Debug, Clone, ToTokens, Spanned)]
pub struct ExprCall {
    /// The name of the function being called.
    pub expr: Box<ast::Expr>,
    /// The arguments of the function call.
    pub args: ast::Parenthesized<ast::Expr, ast::Comma>,
}
