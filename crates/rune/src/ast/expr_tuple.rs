use crate::ast::prelude::*;

#[test]
fn ast_parse() {
    use crate::testing::rt;

    rt::<ast::ExprTuple>("()");
    rt::<ast::ExprTuple>("(1,)");
    rt::<ast::ExprTuple>("(1, \"two\")");
    rt::<ast::ExprTuple>("(1, 2,)");
    rt::<ast::ExprTuple>("(1, 2, foo())");
}

/// An expression to construct a literal tuple.
///
/// * `(<expr>,*)`.
#[derive(Debug, TryClone, PartialEq, Eq, Parse, ToTokens, Spanned)]
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
    pub(crate) fn parse_from_first_expr(
        parser: &mut Parser<'_>,
        attributes: Vec<ast::Attribute>,
        open: ast::OpenParen,
        expr: ast::Expr,
    ) -> Result<Self> {
        Ok(Self {
            attributes,
            items: ast::Parenthesized::parse_from_first(parser, open, expr)?,
        })
    }
}
