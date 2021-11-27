use crate::ast::prelude::*;

/// A block of expressions.
///
/// ```rust
/// use rune::{testing, ast};
///
/// let expr = testing::roundtrip::<ast::ExprBlock>("{}");
/// assert_eq!(expr.block.statements.len(), 0);
///
/// let expr = testing::roundtrip::<ast::ExprBlock>("{ 42 }");
/// assert_eq!(expr.block.statements.len(), 1);
///
/// let expr = testing::roundtrip::<ast::ExprBlock>("#[retry] { 42 }");
/// assert_eq!(expr.block.statements.len(), 1);
/// assert_eq!(expr.attributes.len(), 1);
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
    /// Test if the block produces nothing.
    pub fn produces_nothing(&self) -> bool {
        let mut it = self.statements.iter();

        while let Some(ast::Stmt::Expr(_, semi)) = it.next_back() {
            return semi.is_some();
        }

        true
    }
}

/// Parse implementation for a block.
///
/// # Examples
///
/// ```rust
/// use rune::{testing, ast};
///
/// let block = testing::roundtrip::<ast::Block>("{}");
/// assert_eq!(block.statements.len(), 0);
/// assert!(block.produces_nothing());
///
/// let block = testing::roundtrip::<ast::Block>("{ foo }");
/// assert_eq!(block.statements.len(), 1);
/// assert!(!block.produces_nothing());
///
/// let block = testing::roundtrip::<ast::Block>("{ foo; }");
/// assert_eq!(block.statements.len(), 1);
/// assert!(block.produces_nothing());
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
