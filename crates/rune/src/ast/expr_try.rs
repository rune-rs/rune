use crate::ast;
use crate::Spanned;
use runestick::Span;

/// A try expression `<expr>?`.
#[derive(Debug, Clone)]
pub struct ExprTry {
    /// The expression being awaited.
    pub expr: Box<ast::Expr>,
    /// The try operator.
    pub try_: ast::Try,
}

into_tokens!(ExprTry { expr, try_ });

impl Spanned for ExprTry {
    fn span(&self) -> Span {
        self.expr.span().join(self.try_.span())
    }
}
