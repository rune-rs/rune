use crate::ast;
use crate::{Parse, ParseError, Parser, Spanned, ToTokens};

/// An expression to construct a literal tuple.
///
/// # Examples
///
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::LitTuple>("(1, \"two\")");
/// testing::roundtrip::<ast::LitTuple>("(1, 2,)");
/// testing::roundtrip::<ast::LitTuple>("(1, 2, foo())");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned, Parse)]
pub struct LitTuple {
    /// Items in the tuple.
    pub items: ast::Parenthesized<ast::Expr, ast::Comma>,
}

impl LitTuple {
    /// Start parsing literal tuple from the middle of an expression.
    pub fn parse_from_first_expr(
        parser: &mut Parser<'_>,
        open: ast::OpenParen,
        expr: ast::Expr,
    ) -> Result<Self, ParseError> {
        Ok(Self {
            items: ast::Parenthesized::parse_from_first(parser, open, expr)?,
        })
    }
}
