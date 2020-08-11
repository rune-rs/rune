use crate::error::{ParseError, Result};
use crate::parser::Parser;
use crate::token::{Kind, Token};
use crate::traits::{Parse, Peek};
use runestick::unit::Span;

/// The unit literal `()`.
#[derive(Debug, Clone)]
pub struct LitBool {
    /// The value of the literal.
    pub value: bool,
    /// The token of the literal.
    pub token: Token,
}

impl LitBool {
    /// Get the span of this unit literal.
    pub fn span(&self) -> Span {
        self.token.span
    }
}

/// Parsing a unit literal
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// parse_all::<ast::LitBool>("true").unwrap();
/// parse_all::<ast::LitBool>("false").unwrap();
/// ```
impl Parse for LitBool {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let token = parser.token_next()?;

        let value = match token.kind {
            Kind::True => true,
            Kind::False => false,
            _ => {
                return Err(ParseError::ExpectedBool {
                    span: token.span,
                    actual: token.kind,
                })
            }
        };

        Ok(Self { value, token })
    }
}

impl Peek for LitBool {
    fn peek(p1: Option<Token>, _: Option<Token>) -> bool {
        let p1 = match p1 {
            Some(p1) => p1,
            None => return false,
        };

        match p1.kind {
            Kind::True => true,
            Kind::False => true,
            _ => false,
        }
    }
}
