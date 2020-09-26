use crate::ast;
use crate::{Parse, ParseError, Parser, Spanned, ToTokens};

/// A number literal.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub struct LitVec {
    /// The open bracket.
    pub open: ast::OpenBracket,
    /// Items in the array.
    pub items: Vec<(ast::Expr, Option<ast::Comma>)>,
    /// The close bracket.
    pub close: ast::CloseBracket,
    /// If the entire array is constant.
    #[rune(skip)]
    is_const: bool,
}

impl LitVec {
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
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::LitVec>("[1, \"two\"]");
/// testing::roundtrip::<ast::LitVec>("[1, 2,]");
/// testing::roundtrip::<ast::LitVec>("[1, 2, foo()]");
/// ```
impl Parse for LitVec {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let open = parser.parse()?;

        let mut items = Vec::new();
        let mut is_const = true;

        while !parser.peek::<ast::CloseBracket>()? {
            let expr = parser.parse::<ast::Expr>()?;

            if !expr.is_const() {
                is_const = false;
            }

            let comma = parser.parse::<Option<ast::Comma>>()?;
            let end = comma.is_none();
            items.push((expr, comma));

            if end {
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
