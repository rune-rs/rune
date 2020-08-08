use crate::ast::{CloseParen, OpenParen};
use crate::error::{ParseError, Result};
use crate::parser::Parser;
use crate::token::{Delimiter, Kind, Token};
use crate::traits::{Parse, Peek};
use runestick::unit::Span;

/// The unit literal `()`.
#[derive(Debug, Clone)]
pub struct LitUnit {
    /// The open parenthesis.
    pub open: OpenParen,
    /// The close parenthesis.
    pub close: CloseParen,
}

impl LitUnit {
    /// Get the span of this unit literal.
    pub fn span(&self) -> Span {
        self.open.span().join(self.close.span())
    }
}

/// Parsing a unit literal
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// parse_all::<ast::LitUnit>("()").unwrap();
/// ```
impl Parse for LitUnit {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        Ok(Self {
            open: parser.parse()?,
            close: parser.parse()?,
        })
    }
}

impl Peek for LitUnit {
    fn peek(p1: Option<Token>, p2: Option<Token>) -> bool {
        let (p1, p2) = match (p1, p2) {
            (Some(p1), Some(p2)) => (p1, p2),
            _ => return false,
        };

        matches! {
            (p1.kind, p2.kind),
            (
                Kind::Open {
                    delimiter: Delimiter::Parenthesis,
                },
                Kind::Close {
                    delimiter: Delimiter::Parenthesis,
                },
            )
        }
    }
}
