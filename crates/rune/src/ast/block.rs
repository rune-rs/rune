use crate::ast;
use crate::{Parse, ParseError, Parser};
use runestick::Span;

/// A block of expressions.
#[derive(Debug, Clone)]
pub struct Block {
    /// The close brace.
    pub open: ast::OpenBrace,
    /// Statements in the block.
    pub statements: Vec<ast::Stmt>,
    /// The close brace.
    pub close: ast::CloseBrace,
}

into_tokens!(Block {
    open,
    statements,
    close
});

impl Block {
    /// Get the span of the block.
    pub fn span(&self) -> Span {
        self.open.span().join(self.close.span())
    }

    /// Test if the block expression doesn't produce a value.
    pub fn produces_nothing(&self) -> bool {
        let mut it = self.statements.iter();

        while let Some(stmt) = it.next_back() {
            match stmt {
                ast::Stmt::Expr(..) => return false,
                ast::Stmt::Semi(..) => return true,
                _ => (),
            }
        }

        true
    }

    /// Test if the block is a constant expression.
    pub fn is_const(&self) -> bool {
        for stmt in &self.statements {
            match stmt {
                ast::Stmt::Expr(expr) if !expr.is_const() => return false,
                ast::Stmt::Semi(expr, _) if !expr.is_const() => return false,
                _ => (),
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
/// use rune::{parse_all, ast};
///
/// let block = parse_all::<ast::Block>("{}").unwrap();
/// assert_eq!(block.statements.len(), 0);
/// assert!(block.produces_nothing());
///
/// let block = parse_all::<ast::Block>("{ foo }").unwrap();
/// assert_eq!(block.statements.len(), 1);
/// assert!(!block.produces_nothing());
///
/// let block = parse_all::<ast::Block>("{ foo; }").unwrap();
/// assert_eq!(block.statements.len(), 1);
/// assert!(block.produces_nothing());
///
/// let block = parse_all::<ast::Block>(r#"
///     {
///         let foo = 42;
///         let bar = "string";
///         baz
///     }
/// "#).unwrap();
/// assert_eq!(block.statements.len(), 3);
/// ```
impl Parse for Block {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let mut statements = Vec::new();

        let open = parser.parse()?;

        while !parser.peek::<ast::CloseBrace>()? {
            if ast::Item::peek_as_stmt(parser)? {
                let decl: ast::Item = parser.parse()?;
                statements.push(ast::Stmt::Item(decl));
                continue;
            }

            let expr: ast::Expr = parser.parse()?;

            if parser.peek::<ast::SemiColon>()? {
                statements.push(ast::Stmt::Semi(expr, parser.parse()?));
            } else {
                statements.push(ast::Stmt::Expr(expr));
            }
        }

        let close = parser.parse()?;

        Ok(Block {
            open,
            statements,
            close,
        })
    }
}
