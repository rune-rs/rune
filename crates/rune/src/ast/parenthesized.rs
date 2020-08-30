use crate::ast;
use crate::error::{ParseError, Result};
use crate::parser::Parser;
use crate::traits::{Parse, Peek};
use runestick::unit::Span;

/// Something parenthesized and comma separated `(<T,>*)`.
#[derive(Debug, Clone)]
pub struct Parenthesized<T, S> {
    /// The open parenthesis.
    pub open: ast::OpenParen,
    /// The parenthesized type.
    pub items: Vec<(T, Option<S>)>,
    /// The close parenthesis.
    pub close: ast::CloseParen,
}

impl<T, S> Parenthesized<T, S> {
    /// Access the span of expression.
    pub fn span(&self) -> Span {
        self.open.token.span.join(self.close.token.span)
    }
}

/// Parse function arguments.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// # fn main() -> rune::Result<()> {
/// parse_all::<ast::Parenthesized<ast::Expr, ast::Comma>>("(1, \"two\")")?;
/// parse_all::<ast::Parenthesized<ast::Expr, ast::Comma>>("(1, 2,)")?;
/// parse_all::<ast::Parenthesized<ast::Expr, ast::Comma>>("(1, 2, foo())")?;
/// # Ok(())
/// # }
/// ```
impl<T, S> Parse for Parenthesized<T, S>
where
    T: Parse,
    S: Peek + Parse,
{
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let open = parser.parse()?;

        let mut items = Vec::new();

        while !parser.peek::<ast::CloseParen>()? {
            let expr = parser.parse()?;
            let sep = parser.parse::<Option<S>>()?;
            let is_end = sep.is_none();
            items.push((expr, sep));

            if is_end {
                break;
            }
        }

        let close = parser.parse()?;
        Ok(Self { open, items, close })
    }
}
