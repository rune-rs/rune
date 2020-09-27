use crate::ast;
use crate::{Parse, ParseError, Parser, Spanned, ToTokens};

/// A block of expressions.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub struct Block {
    /// The close brace.
    pub open: ast::OpenBrace,
    /// Statements in the block.
    pub statements: Vec<ast::Stmt>,
    /// The close brace.
    pub close: ast::CloseBrace,
}

impl Block {
    /// Test if the block produces nothing.
    pub fn produces_nothing(&self) -> bool {
        let mut it = self.statements.iter();

        while let Some(stmt) = it.next_back() {
            match stmt {
                ast::Stmt::Item(..) => (),
                ast::Stmt::Expr(..) => return false,
                ast::Stmt::Semi(..) => return true,
            }
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

        while !parser.peek::<ast::CloseBrace>()? {
            statements.push(parser.parse()?);
        }

        let close = parser.parse()?;

        Ok(Block {
            open,
            statements,
            close,
        })
    }
}
