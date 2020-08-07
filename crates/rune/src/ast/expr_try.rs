use crate::ast;
use runestick::unit::Span;

/// A try expression `<expr>?`.
#[derive(Debug, Clone)]
pub struct ExprTry {
    /// The expression being awaited.
    pub expr: Box<ast::Expr>,
    /// The try operator.
    pub try_: ast::Try,
}

impl ExprTry {
    /// Access the span of the expression.
    pub fn span(&self) -> Span {
        self.expr.span().join(self.try_.span())
    }
}
