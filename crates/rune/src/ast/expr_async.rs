use crate::ast;
use crate::{Parse, Spanned, ToTokens};

/// A block of expressions.
///
/// # Examples
///
/// ```rust
/// use rune::{testing, ast};
///
/// let expr = testing::roundtrip::<ast::ExprAsync>("async {}");
/// assert_eq!(expr.block.statements.len(), 0);
///
/// let expr = testing::roundtrip::<ast::ExprAsync>("async { 42 }");
/// assert_eq!(expr.block.statements.len(), 1);
///
/// let expr = testing::roundtrip::<ast::ExprAsync>("#[retry] async { 42 }");
/// assert_eq!(expr.block.statements.len(), 1);
/// assert_eq!(expr.attributes.len(), 1);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Parse, ToTokens, Spanned)]
pub struct ExprAsync {
    /// The attributes for the block.
    #[rune(iter, attributes)]
    pub attributes: Vec<ast::Attribute>,
    /// The `async` keyword.
    pub async_: ast::Async,
    /// The close brace.
    pub block: ast::Block,
}
