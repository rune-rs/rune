use crate::ast;
use crate::{Parse, ParseError, Parser, Spanned, ToTokens};

/// An if condition.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub enum Condition {
    /// A regular expression.
    Expr(Box<ast::Expr>),
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
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let token = parser.token_peek_eof()?;

        Ok(match token.kind {
            ast::Kind::Let => {
                Self::ExprLet(Box::new(ast::ExprLet::parse_without_eager_brace(parser)?))
            }
            _ => Self::Expr(Box::new(ast::Expr::parse_without_eager_brace(parser)?)),
        })
    }
}
