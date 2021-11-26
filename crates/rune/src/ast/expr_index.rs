use crate::ast::prelude::*;

/// An index get operation `<target>[<index>]`.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
#[non_exhaustive]
pub struct ExprIndex {
    /// Attributes associated with expression.
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The target of the index set.
    pub target: ast::Expr,
    /// The opening bracket.
    pub open: T!['['],
    /// The indexing expression.
    pub index: ast::Expr,
    /// The closening bracket.
    pub close: T![']'],
}

expr_parse!(Index, ExprIndex, "index expression");
