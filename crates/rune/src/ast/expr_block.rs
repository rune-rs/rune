use crate::ast::{CloseBrace, Expr, OpenBrace, SemiColon};
use crate::error::ParseError;
use crate::parser::Parser;
use crate::traits::Parse;
use st::unit::Span;

/// A block of expressions.
#[derive(Debug, Clone)]
pub struct ExprBlock {
    /// The close brace.
    pub open: OpenBrace,
    /// Expressions in the block.
    pub exprs: Vec<(Expr, Option<SemiColon>)>,
    /// Test if the expression is trailing.
    pub trailing_expr: Option<Box<Expr>>,
    /// The close brace.
    pub close: CloseBrace,
}

impl ExprBlock {
    /// Get the span of the block.
    pub fn span(&self) -> Span {
        self.open.span().join(self.close.span())
    }

    /// Test if the block is empty.
    pub fn produces_nothing(&self) -> bool {
        match &self.trailing_expr {
            Some(trailing) => trailing.produces_nothing(),
            None => true,
        }
    }

    /// ExprBlock is constant if a trailing expression exists and is all literal.
    pub fn is_const(&self) -> bool {
        match &self.trailing_expr {
            Some(trailing) => trailing.is_const(),
            None => false,
        }
    }
}

/// Parse implementation for a block.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// # fn main() -> anyhow::Result<()> {
/// let block = parse_all::<ast::ExprBlock>("{}")?.item;
/// assert_eq!(block.exprs.len(), 0);
/// assert!(block.trailing_expr.is_none());
///
/// let block = parse_all::<ast::ExprBlock>("{ foo }")?.item;
/// assert_eq!(block.exprs.len(), 0);
/// assert!(block.trailing_expr.is_some());
///
/// let block = parse_all::<ast::ExprBlock>("{ foo; }")?.item;
/// assert_eq!(block.exprs.len(), 1);
/// assert!(block.trailing_expr.is_none());
///
/// let block = parse_all::<ast::ExprBlock>(r#"
///     {
///         let foo = 42;
///         let bar = "string";
///         baz
///     }
/// "#)?.item;
/// assert_eq!(block.exprs.len(), 2);
/// assert!(block.trailing_expr.is_some());
/// # Ok(())
/// # }
/// ```
impl Parse for ExprBlock {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let mut exprs = Vec::new();

        let open = parser.parse()?;
        let mut trailing_expr = None;

        // Last expression is of a type that evaluates to a value.
        let mut last_expr_with_value = false;

        while !parser.peek::<CloseBrace>()? {
            last_expr_with_value = false;
            let expr: Expr = parser.parse()?;

            if parser.peek::<SemiColon>()? {
                exprs.push((expr, Some(parser.parse::<SemiColon>()?)));
                continue;
            }

            // expressions where it's allowed not to have a trailing
            // semi-colon.
            match &expr {
                Expr::ExprWhile(..) | Expr::ExprLoop(..) | Expr::ExprFor(..) => {
                    exprs.push((expr, None));
                    continue;
                }
                Expr::ExprIf(expr_if) => {
                    if expr_if.produces_nothing() {
                        exprs.push((expr, None));
                    } else {
                        last_expr_with_value = true;
                        exprs.push((expr, None));
                    }

                    continue;
                }
                Expr::ExprMatch(expr_match) => {
                    if expr_match.produces_nothing() {
                        exprs.push((expr, None));
                    } else {
                        last_expr_with_value = true;
                        exprs.push((expr, None));
                    }

                    continue;
                }
                _ => (),
            }

            trailing_expr = Some(Box::new(expr));
            break;
        }

        if last_expr_with_value {
            trailing_expr = exprs.pop().map(|(expr, _)| Box::new(expr));
        }

        let close = parser.parse()?;

        Ok(ExprBlock {
            open,
            exprs,
            trailing_expr,
            close,
        })
    }
}
