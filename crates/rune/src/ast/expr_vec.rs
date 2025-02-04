use crate::ast::prelude::*;

#[test]
#[cfg(not(miri))]
fn ast_parse() {
    rt::<ast::ExprVec>("[1, \"two\"]");
    rt::<ast::ExprVec>("[1, 2,]");
    rt::<ast::ExprVec>("[1, 2, foo()]");
}

/// A literal vector.
///
/// * `[<expr>,*]`
#[derive(Debug, TryClone, PartialEq, Eq, Parse, ToTokens, Spanned)]
#[non_exhaustive]
pub struct ExprVec {
    /// Attributes associated with vector.
    #[rune(iter, meta)]
    pub attributes: Vec<ast::Attribute>,
    /// Items in the vector.
    pub items: ast::Bracketed<ast::Expr, T![,]>,
}
