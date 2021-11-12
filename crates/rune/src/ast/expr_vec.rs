use crate::ast::prelude::*;

/// A literal vector.
///
/// # Examples
///
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::ExprVec>("[1, \"two\"]");
/// testing::roundtrip::<ast::ExprVec>("[1, 2,]");
/// testing::roundtrip::<ast::ExprVec>("[1, 2, foo()]");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Parse, ToTokens, Spanned)]
pub struct ExprVec {
    /// Attributes associated with vector.
    #[rune(iter, meta)]
    pub attributes: Vec<ast::Attribute>,
    /// Items in the vector.
    pub items: ast::Bracketed<ast::Expr, T![,]>,
}
