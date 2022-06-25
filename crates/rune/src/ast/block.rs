use crate::ast::prelude::*;

/// A block of expressions.
///
/// ```
/// use rune::{ast, testing};
///
/// let expr = testing::roundtrip::<ast::ExprBlock>("{}");
/// assert_eq!(expr.block.statements.len(), 0);
///
/// let expr = testing::roundtrip::<ast::ExprBlock>("{ 42 }");
/// assert_eq!(expr.block.statements.len(), 1);
///
/// let block = testing::roundtrip::<ast::Block>("{ foo }");
/// assert_eq!(block.statements.len(), 1);
///
/// let block = testing::roundtrip::<ast::Block>("{ foo; }");
/// assert_eq!(block.statements.len(), 1);
///
/// let expr = testing::roundtrip::<ast::ExprBlock>("#[retry] { 42 }");
/// assert_eq!(expr.block.statements.len(), 1);
/// assert_eq!(expr.attributes.len(), 1);
///
/// let block = testing::roundtrip::<ast::Block>(r#"
///     {
///         let foo = 42;
///         let bar = "string";
///         baz
///     }
/// "#);
///
/// assert_eq!(block.statements.len(), 3);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned, Opaque)]
#[non_exhaustive]
pub struct Block {
    /// The unique identifier for the block expression.
    #[rune(id)]
    pub(crate) id: Id,
    /// The close brace.
    pub open: T!['{'],
    /// Statements in the block.
    pub statements: Vec<ast::Stmt>,
    /// The close brace.
    pub close: T!['}'],
}

impl Block {
    /// Test if the block doesn't produce anything. Which is when the last
    /// element is either a non-expression or is an expression terminated by a
    /// semi.
    pub(crate) fn produces_nothing(&self) -> bool {
        matches!(self.statements.last(), Some(ast::Stmt::Semi(..)) | None)
    }
}

impl Parse for Block {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let mut statements = Vec::new();

        let open = parser.parse()?;

        while !parser.peek::<T!['}']>()? {
            statements.push(parser.parse()?);
        }

        let close = parser.parse()?;

        Ok(Block {
            id: Default::default(),
            open,
            statements,
            close,
        })
    }
}
