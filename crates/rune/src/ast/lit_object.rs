use crate::ast::{CloseBrace, Colon, Comma, Expr, LitStr, StartObject};
use crate::error::{ParseError, Result};
use crate::parser::Parser;
use crate::traits::Parse;
use st::unit::Span;

/// A number literal.
#[derive(Debug, Clone)]
pub struct LitObject {
    /// The open bracket.
    pub open: StartObject,
    /// Items in the object declaration.
    pub items: Vec<(LitStr, Colon, Expr)>,
    /// The close bracket.
    pub close: CloseBrace,
    /// Indicates if the object is completely literal and cannot have side
    /// effects.
    is_const: bool,
}

impl LitObject {
    /// Access the span of the expression.
    pub fn span(&self) -> Span {
        self.open.span().join(self.close.span())
    }

    /// Test if the entire expression is constant.
    pub fn is_const(&self) -> bool {
        self.is_const
    }
}

/// Parse an object literal.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// # fn main() -> anyhow::Result<()> {
/// parse_all::<ast::LitObject>("#{\"foo\": 42}")?;
/// parse_all::<ast::LitObject>("#{\"foo\": 42,}")?;
/// # Ok(())
/// # }
/// ```
impl Parse for LitObject {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let open = parser.parse()?;

        let mut items = Vec::new();

        let mut is_const = true;

        while !parser.peek::<CloseBrace>()? {
            let key = parser.parse()?;
            let colon = parser.parse()?;
            let expr = parser.parse::<Expr>()?;

            if !expr.is_const() {
                is_const = false;
            }

            items.push((key, colon, expr));

            if parser.peek::<Comma>()? {
                parser.parse::<Comma>()?;
            } else {
                break;
            }
        }

        let close = parser.parse()?;
        Ok(Self {
            open,
            items,
            close,
            is_const,
        })
    }
}
