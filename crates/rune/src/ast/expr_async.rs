use crate::ast;
use crate::{Parse, ParseError, Parser, Spanned};
use runestick::Span;

/// A block of expressions.
#[derive(Debug, Clone)]
pub struct ExprAsync {
    /// The `async` keyword.
    pub async_: ast::Async,
    /// The close brace.
    pub block: ast::Block,
}

into_tokens!(ExprAsync { async_, block });

impl Spanned for ExprAsync {
    fn span(&self) -> Span {
        self.async_.span().join(self.block.span())
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
/// ```
impl Parse for ExprAsync {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        Ok(Self {
            async_: parser.parse()?,
            block: parser.parse()?,
        })
    }
}
