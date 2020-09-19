use crate::ast;
use crate::Ast;
use runestick::Span;

/// The field being accessed.
#[derive(Debug, Clone, Ast)]
pub enum ExprField {
    /// An identifier.
    Ident(ast::Ident),
    /// A literal number.
    LitNumber(ast::LitNumber),
}

/// A field access `<expr>.<field>`.
#[derive(Debug, Clone, Ast)]
pub struct ExprFieldAccess {
    /// The expr where the field is being accessed.
    pub expr: Box<ast::Expr>,
    /// The parsed dot separator.
    pub dot: ast::Dot,
    /// The field being accessed.
    pub expr_field: ExprField,
}

impl crate::Spanned for ExprFieldAccess {
    fn span(&self) -> Span {
        self.expr.span().join(self.expr_field.span())
    }
}
