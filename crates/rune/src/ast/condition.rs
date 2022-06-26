use crate::ast::prelude::*;

/// The condition in an if statement.
///
/// # Examples
///
/// ```
/// use rune::{ast, testing};
///
/// testing::roundtrip::<ast::Condition>("true");
/// testing::roundtrip::<ast::Condition>("let [a, ..] = v");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
#[non_exhaustive]
pub enum Condition {
    /// A regular expression.
    Expr(ast::Expr),
    /// A pattern match.
    ExprLet(ast::ExprLet),
}

impl Parse for Condition {
    fn parse(p: &mut Parser) -> Result<Self, ParseError> {
        Ok(match p.nth(0)? {
            K![let] => Self::ExprLet(ast::ExprLet::parse_without_eager_brace(p)?),
            _ => Self::Expr(ast::Expr::parse_without_eager_brace(p)?),
        })
    }
}
