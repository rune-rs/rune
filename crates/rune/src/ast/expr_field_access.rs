use crate::ast;
use crate::Spanned;
use runestick::Span;

impl_enum_ast! {
    /// The field being accessed.
    pub enum ExprField {
        /// An identifier.
        Ident(ast::Ident),
        /// A literal number.
        LitNumber(ast::LitNumber),
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

impl Spanned for ExprFieldAccess {
    fn span(&self) -> Span {
        self.expr.span().join(self.expr_field.span())
    }
}
