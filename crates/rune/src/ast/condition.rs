use crate::ast;
use crate::error::ParseError;
use crate::parser::Parser;
use crate::traits::Parse;
use runestick::Span;

/// An if condition.
#[derive(Debug, Clone)]
pub enum Condition {
    /// A regular expression.
    Expr(Box<ast::Expr>),
    /// A pattern match.
    ExprLet(Box<ast::ExprLet>),
}

into_tokens_enum!(Condition { Expr, ExprLet });

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
