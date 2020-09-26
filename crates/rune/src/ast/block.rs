use crate::ast;
use crate::{OptionSpanned as _, Parse, ParseError, ParseErrorKind, Parser, Spanned, ToTokens};

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
        let mut must_be_last = None;

        while !parser.peek::<ast::CloseBrace>()? {
            let attributes = parser.parse()?;
            let visibility = parser.parse()?;

            if ast::Item::peek_as_stmt(parser)? {
                let decl: ast::Item = ast::Item::parse_with_meta(parser, attributes, visibility)?;

                if let Some(span) = must_be_last {
                    return Err(ParseError::new(
                        span,
                        ParseErrorKind::ExpectedBlockSemiColon {
                            followed_span: decl.span(),
                        },
                    ));
                }

                statements.push(ast::Stmt::Item(decl));
                continue;
            }

            if let Some(span) = visibility.option_span() {
                return Err(ParseError::new(
                    span,
                    ParseErrorKind::UnsupportedExprVisibility,
                ));
            }

            let expr: ast::Expr = ast::Expr::parse_primary_with_attributes(parser, attributes)?;

            if let Some(span) = must_be_last {
                return Err(ParseError::new(
                    span,
                    ParseErrorKind::ExpectedBlockSemiColon {
                        followed_span: expr.span(),
                    },
                ));
            }

            if parser.peek::<ast::SemiColon>()? {
                statements.push(ast::Stmt::Semi(expr, parser.parse()?));
            } else {
                if expr.needs_semi() {
                    must_be_last = Some(expr.span());
                }

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
