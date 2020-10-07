use crate::ast;
use crate::{Parse, Spanned, ToTokens};

/// A literal vector.
///
/// # Examples
///
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::LitVec>("[1, \"two\"]");
/// testing::roundtrip::<ast::LitVec>("[1, 2,]");
/// testing::roundtrip::<ast::LitVec>("[1, 2, foo()]");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Parse, ToTokens, Spanned)]
pub struct LitVec {
    /// Items in the vector.
    pub items: ast::Bracketed<ast::Expr, T![,]>,
}
