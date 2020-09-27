use crate::ast;
use crate::{Spanned, ToTokens};

/// The field being accessed.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub enum ExprField {
    /// An identifier.
    Ident(ast::Ident),
    /// A literal number.
    LitNumber(ast::LitNumber),
}

/// A field access `<expr>.<field>`.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub struct ExprFieldAccess {
    /// Attributes associated with expression.
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The expr where the field is being accessed.
    pub expr: Box<ast::Expr>,
    /// The parsed dot separator.
    pub dot: ast::Dot,
    /// The field being accessed.
    pub expr_field: ExprField,
}
