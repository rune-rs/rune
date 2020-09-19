use crate::ast;
use crate::{Ast, Spanned};
use runestick::Span;

/// A function call `<expr>(<args>)`.
#[derive(Debug, Clone, Ast)]
pub struct ExprCall {
    /// The name of the function being called.
    pub expr: Box<ast::Expr>,
    /// The arguments of the function call.
    pub args: ast::Parenthesized<ast::Expr, ast::Comma>,
}

impl Spanned for ExprCall {
    fn span(&self) -> Span {
        self.expr.span().join(self.args.span())
    }
}
