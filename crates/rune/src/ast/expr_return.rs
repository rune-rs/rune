use crate::ast;
use crate::{Ast, Parse, Spanned};
use runestick::Span;

/// A return statement `return [expr]`.
#[derive(Debug, Clone, Ast, Parse)]
pub struct ExprReturn {
    /// The return token.
    pub return_: ast::Return,
    /// An optional expression to return.
    pub expr: Option<Box<ast::Expr>>,
}

impl Spanned for ExprReturn {
    fn span(&self) -> Span {
        if let Some(expr) = &self.expr {
            self.return_.span().join(expr.span())
        } else {
            self.return_.span()
        }
    }
}
