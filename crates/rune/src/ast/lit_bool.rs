use crate::ast;
use crate::{Parse, ParseError, ParseErrorKind, Parser, Peek, Spanned};
use runestick::Span;

/// The unit literal `()`.
#[derive(Debug, Clone)]
pub struct LitBool {
    /// The value of the literal.
    pub value: bool,
    /// The token of the literal.
    pub token: ast::Token,
}

into_tokens!(LitBool { token });

impl Spanned for LitBool {
    fn span(&self) -> Span {
        self.token.span()
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
            ast::Kind::True => true,
            ast::Kind::False => false,
            _ => {
                return Err(ParseError::new(
                    token,
                    ParseErrorKind::ExpectedBool { actual: token.kind },
                ));
            }
        };

        Ok(Self { value, token })
    }
}

impl Peek for LitBool {
    fn peek(p1: Option<ast::Token>, _: Option<ast::Token>) -> bool {
        let p1 = match p1 {
            Some(p1) => p1,
            None => return false,
        };

        match p1.kind {
            ast::Kind::True => true,
            ast::Kind::False => true,
            _ => false,
        }
    }
}
