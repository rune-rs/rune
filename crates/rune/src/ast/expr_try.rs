use crate::ast::prelude::*;

#[test]
#[cfg(not(miri))]
fn ast_parse() {
    rt::<ast::ExprTry>("42?");
    rt::<ast::ExprTry>("foo()?");
}

/// A try expression.
///
/// * `<expr>?`.
#[derive(Debug, TryClone, PartialEq, Eq, ToTokens, Spanned)]
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
