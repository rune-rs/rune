use crate::ast;
use crate::{Parse, Spanned, ToTokens};

/// A return statement `return [expr]`.
#[derive(Debug, Clone, ToTokens, Parse, Spanned)]
pub struct ExprReturn {
    /// The return token.
    pub return_: ast::Return,
    /// An optional expression to return.
    #[rune(iter)]
    pub expr: Option<Box<ast::Expr>>,
}
