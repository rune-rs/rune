use crate::ast::utils;
use crate::error::{ParseError, ResolveError};
use crate::parser::Parser;
use crate::source::Source;
use crate::token::{Kind, Token};
use crate::traits::{Parse, Resolve};
use stk::unit::Span;

/// A number literal.
#[derive(Debug, Clone)]
pub struct LitChar {
    /// The token corresponding to the literal.
    pub token: Token,
}

impl LitChar {
    /// Access the span of the expression.
    pub fn span(&self) -> Span {
        self.token.span
    }
}

/// Parse a number literal.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// # fn main() -> anyhow::Result<()> {
/// parse_all::<ast::LitChar>("'a'")?;
/// parse_all::<ast::LitChar>("'\\0'")?;
/// parse_all::<ast::LitChar>("'\\n'")?;
/// parse_all::<ast::LitChar>("'\\r'")?;
/// parse_all::<ast::LitChar>("'\\''")?;
/// # Ok(())
/// # }
/// ```
impl Parse for LitChar {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let token = parser.token_next()?;

        Ok(match token.kind {
            Kind::LitChar => LitChar { token },
            _ => {
                return Err(ParseError::ExpectedCharError {
                    actual: token.kind,
                    span: token.span,
                })
            }
        })
    }
}

impl<'a> Resolve<'a> for LitChar {
    type Output = char;

    fn resolve(&self, source: Source<'a>) -> Result<char, ResolveError> {
        let span = self.token.span;
        let string = source.source(span.narrow(1))?;
        let mut it = string
            .char_indices()
            .map(|(n, c)| (span.start + n, c))
            .peekable();

        let (n, c) = match it.next() {
            Some(c) => c,
            None => {
                return Err(ResolveError::BadCharacterLiteral { span });
            }
        };

        let c = match c {
            '\\' => utils::parse_char_escape(span.with_start(n), &mut it)?,
            c => c,
        };

        // Too many characters in literal.
        if it.next().is_some() {
            return Err(ResolveError::BadCharacterLiteral { span });
        }

        Ok(c)
    }
}
