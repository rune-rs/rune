use crate::ast;
use crate::{Spanned, ToTokens};

/// An index set operation `<target>[<index>] = <value>`.
#[derive(Debug, Clone, ToTokens, Spanned)]
pub struct ExprIndexSet {
    /// The target of the index set.
    pub target: Box<ast::Expr>,
    /// The opening bracket.
    pub open: ast::OpenBracket,
    /// The indexing expression.
    pub index: Box<ast::Expr>,
    /// The closening bracket.
    pub close: ast::CloseBracket,
    /// The equals sign.
    pub eq: ast::Eq,
    /// The value expression we are assigning.
    pub value: Box<ast::Expr>,
}
