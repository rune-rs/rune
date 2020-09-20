use crate::ast;
use crate::{Parse, ParseError, Parser, Spanned, ToTokens};

/// A block of expressions.
#[derive(Debug, Clone, ToTokens, Spanned)]
pub struct ExprAsync {
    /// The attributes for the block.
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The `async` keyword.
    pub async_: ast::Async,
    /// The close brace.
    pub block: ast::Block,
}

impl ExprAsync {
    /// Parse an async block expression attaching the given attributes to the
    /// block
    pub fn parse_with_attributes(
        parser: &mut Parser<'_>,
        attributes: Vec<ast::Attribute>,
    ) -> Result<Self, ParseError> {
        Ok(Self {
            attributes,
            async_: parser.parse()?,
            block: parser.parse()?,
        })
    }
}

/// Parse implementation for a block.
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
impl Parse for ExprAsync {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let attributes = parser.parse()?;
        Self::parse_with_attributes(parser, attributes)
    }
}
