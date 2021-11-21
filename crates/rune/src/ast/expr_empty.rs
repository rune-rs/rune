use crate::ast::prelude::*;

/// A prioritized expression group without delimiters `<expr>`.
///
/// These groups are only produced during internal desugaring. Most notably
/// through the use of template literals.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub struct ExprEmpty {
    /// Attributes associated with expression.
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The open parenthesis.
    pub open: ast::OpenEmpty,
    /// The grouped expression.
    pub expr: ast::Expr,
    /// The close parenthesis.
    pub close: ast::CloseEmpty,
}

expr_parse!(Empty, ExprEmpty, "empty group expression");
