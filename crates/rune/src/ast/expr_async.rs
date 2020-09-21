use crate::ast;
use crate::{Parse, Spanned, ToTokens};

/// A block of expressions.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// let expr = parse_all::<ast::ExprAsync>("async {}").unwrap();
/// assert_eq!(expr.block.statements.len(), 0);
/// assert!(expr.block.produces_nothing());
///
/// let expr = parse_all::<ast::ExprAsync>("async { 42 }").unwrap();
/// assert_eq!(expr.block.statements.len(), 1);
/// assert!(!expr.block.produces_nothing());
///
/// let expr = parse_all::<ast::ExprAsync>("#[retry] async { 42 }").unwrap();
/// assert_eq!(expr.block.statements.len(), 1);
/// assert!(!expr.block.produces_nothing());
/// assert_eq!(expr.attributes.len(), 1);
/// ```
#[derive(Debug, Clone, Parse, ToTokens, Spanned)]
pub struct ExprAsync {
    /// The attributes for the block.
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The `async` keyword.
    pub async_: ast::Async,
    /// The close brace.
    pub block: ast::Block,
}
