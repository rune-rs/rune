use crate::ast::prelude::*;

/// An if condition.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub enum Condition {
    /// A regular expression.
    Expr(ast::Expr),
    /// A pattern match.
    ExprLet(Box<ast::ExprLet>),
}

/// Parse a condition.
///
/// # Examples
///
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::Condition>("true");
/// testing::roundtrip::<ast::Condition>("let [a, ..] = v");
/// ```
impl Parse for Condition {
    fn parse(p: &mut Parser) -> Result<Self, ParseError> {
        Ok(match p.nth(0)? {
            K![let] => Self::ExprLet(Box::new(ast::ExprLet::parse_without_eager_brace(p)?)),
            _ => Self::Expr(ast::Expr::parse_without_eager_brace(p)?),
        })
    }
}
