use crate::ast::prelude::*;

/// An expression to construct a literal tuple.
///
/// # Examples
///
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::ExprTuple>("(1, \"two\")");
/// testing::roundtrip::<ast::ExprTuple>("(1, 2,)");
/// testing::roundtrip::<ast::ExprTuple>("(1, 2, foo())");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Parse, ToTokens, Spanned)]
#[non_exhaustive]
pub struct ExprTuple {
    /// Attributes associated with tuple.
    #[rune(iter, meta)]
    pub attributes: Vec<ast::Attribute>,
    /// Items in the tuple.
    pub items: ast::Parenthesized<ast::Expr, T![,]>,
}

impl ExprTuple {
    /// Start parsing literal tuple from the middle of an expression.
    pub fn parse_from_first_expr(
        parser: &mut Parser<'_>,
        attributes: Vec<ast::Attribute>,
        open: ast::OpenParen,
        expr: ast::Expr,
    ) -> Result<Self, ParseError> {
        Ok(Self {
            attributes,
            items: ast::Parenthesized::parse_from_first(parser, open, expr)?,
        })
    }
}
