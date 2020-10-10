use crate::ast;
use crate::{Spanned, ToTokens};

/// A block expression.
///
/// # Examples
///
/// ```rust
/// use rune::{testing, ast};
///
/// let expr = testing::roundtrip::<ast::ExprBlock>("async {}");
/// assert_eq!(expr.block.statements.len(), 0);
///
/// let expr = testing::roundtrip::<ast::ExprBlock>("async { 42 }");
/// assert_eq!(expr.block.statements.len(), 1);
///
/// let expr = testing::roundtrip::<ast::ExprBlock>("#[retry] async { 42 }");
/// assert_eq!(expr.block.statements.len(), 1);
/// assert_eq!(expr.attributes.len(), 1);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub struct ExprBlock {
    /// The attributes for the block.
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The optional async token.
    #[rune(iter)]
    pub async_token: Option<T![async]>,
    /// The close brace.
    pub block: ast::Block,
}

expr_parse!(Block, ExprBlock, "block expression");
