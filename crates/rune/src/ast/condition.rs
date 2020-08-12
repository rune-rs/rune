use crate::ast;
use crate::error::ParseError;
use crate::parser::Parser;
use crate::token::Kind;
use crate::traits::Parse;
use runestick::unit::Span;

/// An if condition.
#[derive(Debug, Clone)]
pub enum Condition {
    /// A regular expression.
    Expr(Box<ast::Expr>),
    /// A pattern match.
    ExprLet(ast::ExprLet),
}

impl Condition {
    /// Access the span of the expression.
    pub fn span(&self) -> Span {
        match self {
            Self::Expr(expr) => expr.span(),
            Self::ExprLet(expr_let) => expr_let.span(),
        }
    }
}

/// Parse a condition.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// # fn main() -> rune::Result<()> {
/// parse_all::<ast::Condition>("true")?;
/// parse_all::<ast::Condition>("let [a, ..] = v")?;
/// # Ok(())
/// # }
/// ```
impl Parse for Condition {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let token = parser.token_peek_eof()?;

        Ok(match token.kind {
            Kind::Let => Self::ExprLet(ast::ExprLet::parse_without_eager_brace(parser)?),
            _ => Self::Expr(Box::new(ast::Expr::parse_without_eager_brace(parser)?)),
        })
    }
}
