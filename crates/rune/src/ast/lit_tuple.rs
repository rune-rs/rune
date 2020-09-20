use crate::ast;
use crate::{Parse, ParseError, Parser, Spanned, ToTokens};

/// An expression to construct a literal tuple.
#[derive(Debug, Clone, ToTokens, Spanned)]
pub struct LitTuple {
    /// The open bracket.
    pub open: ast::OpenParen,
    /// Items in the tuple.
    pub items: Vec<(ast::Expr, Option<ast::Comma>)>,
    /// The close bracket.
    pub close: ast::CloseParen,
    /// If the entire tuple is constant.
    #[rune(skip)]
    is_const: bool,
}

impl LitTuple {
    /// If the tuple is constant.
    pub fn is_const(&self) -> bool {
        self.is_const
    }

    /// Start parsing literal tuple from the middle of an expression.
    pub fn parse_from_first_expr(
        parser: &mut Parser<'_>,
        open: ast::OpenParen,
        mut expr: ast::Expr,
    ) -> Result<Self, ParseError> {
        let mut items = Vec::new();
        let mut is_const = true;

        loop {
            if !expr.is_const() {
                is_const = false;
            }

            let comma = if parser.peek::<ast::Comma>()? {
                Some(parser.parse::<ast::Comma>()?)
            } else {
                None
            };

            let is_end = comma.is_none();
            items.push((expr, comma));

            if is_end || parser.peek::<ast::CloseParen>()? {
                break;
            }

            expr = parser.parse::<ast::Expr>()?;
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

/// Parse a tuple literal.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// parse_all::<ast::LitTuple>("(1, \"two\")").unwrap();
/// parse_all::<ast::LitTuple>("(1, 2,)").unwrap();
/// parse_all::<ast::LitTuple>("(1, 2, foo())").unwrap();
/// ```
impl Parse for LitTuple {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let open = parser.parse()?;

        let mut items = Vec::new();
        let mut is_const = true;

        while !parser.peek::<ast::CloseParen>()? {
            let expr = parser.parse::<ast::Expr>()?;

            if !expr.is_const() {
                is_const = false;
            }

            let comma = if parser.peek::<ast::Comma>()? {
                Some(parser.parse::<ast::Comma>()?)
            } else {
                None
            };

            let is_end = comma.is_none();
            items.push((expr, comma));

            if is_end {
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
