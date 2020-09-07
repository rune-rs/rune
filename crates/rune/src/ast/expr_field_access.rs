use crate::ast;
use runestick::Span;

/// The field being accessed.
#[derive(Debug, Clone)]
pub enum ExprField {
    /// An identifier.
    Ident(ast::Ident),
    /// A literal number.
    LitNumber(ast::LitNumber),
}

into_tokens_enum!(ExprField { Ident, LitNumber });

impl ExprField {
    /// Access the span of the expression.
    pub fn span(&self) -> Span {
        match self {
            Self::Ident(ident) => ident.span(),
            Self::LitNumber(n) => n.span(),
        }
    }
}

/// A field access `<expr>.<field>`.
#[derive(Debug, Clone)]
pub struct ExprFieldAccess {
    /// The expr where the field is being accessed.
    pub expr: Box<ast::Expr>,
    /// The parsed dot separator.
    pub dot: ast::Dot,
    /// The field being accessed.
    pub expr_field: ExprField,
}

into_tokens!(ExprFieldAccess {
    expr,
    dot,
    expr_field
});

impl ExprFieldAccess {
    /// Access the span of the expression.
    pub fn span(&self) -> Span {
        self.expr.span().join(self.expr_field.span())
    }
}
