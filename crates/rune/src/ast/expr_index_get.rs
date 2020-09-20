use crate::ast;
use crate::{Spanned, ToTokens};

/// An index get operation `<target>[<index>]`.
#[derive(Debug, Clone, ToTokens, Spanned)]
pub struct ExprIndexGet {
    /// The target of the index set.
    pub target: Box<ast::Expr>,
    /// The opening bracket.
    pub open: ast::OpenBracket,
    /// The indexing expression.
    pub index: Box<ast::Expr>,
    /// The closening bracket.
    pub close: ast::CloseBracket,
}
