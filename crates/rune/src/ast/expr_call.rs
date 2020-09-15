use crate::ast;
use crate::Spanned;
use runestick::Span;

/// A function call `<expr>(<args>)`.
#[derive(Debug, Clone)]
pub struct ExprCall {
    /// The name of the function being called.
    pub expr: Box<ast::Expr>,
    /// The arguments of the function call.
    pub args: ast::Parenthesized<ast::Expr, ast::Comma>,
}

into_tokens!(ExprCall { expr, args });

impl Spanned for ExprCall {
    fn span(&self) -> Span {
        self.expr.span().join(self.args.span())
    }
}
