use crate::ast;
use crate::{Parse, ParseError, Parser};

impl_enum_ast! {
    /// An if condition.
    pub enum Condition {
        /// A regular expression.
        Expr(Box<ast::Expr>),
        /// A pattern match.
        ExprLet(Box<ast::ExprLet>),
    }
}

/// Parse a condition.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// parse_all::<ast::Condition>("true").unwrap();
/// parse_all::<ast::Condition>("let [a, ..] = v").unwrap();
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
