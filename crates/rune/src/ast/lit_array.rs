use crate::ast::{CloseBracket, Comma, Expr, OpenBracket};
use crate::error::{ParseError, Result};
use crate::parser::Parser;
use crate::traits::Parse;
use stk::unit::Span;

/// A number literal.
#[derive(Debug, Clone)]
pub struct LitArray {
    /// The open bracket.
    pub open: OpenBracket,
    /// Items in the array.
    pub items: Vec<Expr>,
    /// The close bracket.
    pub close: CloseBracket,
    /// If the entire array is constant.
    is_const: bool,
}

impl LitArray {
    /// Access the span of the expression.
    pub fn span(&self) -> Span {
        self.open.span().join(self.close.span())
    }

    /// Test if the entire expression is constant.
    pub fn is_const(&self) -> bool {
        self.is_const
    }
}

/// Parse an array literal.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// # fn main() -> rune::Result<()> {
/// parse_all::<ast::LitArray>("[1, \"two\"]").unwrap();
/// parse_all::<ast::LitArray>("[1, 2,]").unwrap();
/// parse_all::<ast::LitArray>("[1, 2, foo()]").unwrap();
/// # Ok(())
/// # }
/// ```
impl Parse for LitArray {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let open = parser.parse()?;

        let mut items = Vec::new();
        let mut is_const = true;

        while !parser.peek::<CloseBracket>()? {
            let expr = parser.parse::<Expr>()?;

            if !expr.is_const() {
                is_const = false;
            }

            items.push(expr);

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
