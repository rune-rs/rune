use crate::ast;
use crate::{Ast, Parse, Spanned};

/// A prioritized expression group `(<expr>)`.
#[derive(Debug, Clone, Ast, Parse, Spanned)]
pub struct ExprGroup {
    /// The open parenthesis.
    pub open: ast::OpenParen,
    /// The grouped expression.
    pub expr: Box<ast::Expr>,
    /// The close parenthesis.
    pub close: ast::CloseParen,
}

impl ExprGroup {
    /// Check if expression is empty.
    pub fn produces_nothing(&self) -> bool {
        self.expr.produces_nothing()
    }
}
