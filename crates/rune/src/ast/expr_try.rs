use crate::ast::prelude::*;

/// A try expression `<expr>?`.
///
/// ```
/// use rune::{ast, testing};
///
/// testing::roundtrip::<ast::ExprTry>("42?");
/// testing::roundtrip::<ast::ExprTry>("foo()?");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
#[non_exhaustive]
pub struct ExprTry {
    /// Attributes associated with expression.
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The expression being awaited.
    pub expr: Box<ast::Expr>,
    /// The try operator `?`.
    pub try_token: T![?],
}

expr_parse!(Try, ExprTry, "try expression");
