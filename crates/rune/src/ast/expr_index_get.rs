use crate::ast;
use crate::{Ast, Spanned};

/// An index get operation `<target>[<index>]`.
#[derive(Debug, Clone, Ast, Spanned)]
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
