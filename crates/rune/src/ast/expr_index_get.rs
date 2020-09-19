use crate::ast;
use crate::{Ast, Spanned};
use runestick::Span;

/// An index get operation `<target>[<index>]`.
#[derive(Debug, Clone, Ast)]
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

impl Spanned for ExprIndexGet {
    fn span(&self) -> Span {
        self.target.span().join(self.close.span())
    }
}
