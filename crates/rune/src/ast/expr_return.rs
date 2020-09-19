use crate::ast;
use crate::{Ast, Parse, Spanned};

/// A return statement `return [expr]`.
#[derive(Debug, Clone, Ast, Parse, Spanned)]
pub struct ExprReturn {
    /// The return token.
    pub return_: ast::Return,
    /// An optional expression to return.
    #[spanned(last)]
    pub expr: Option<Box<ast::Expr>>,
}
