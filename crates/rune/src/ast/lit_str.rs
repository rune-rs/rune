use crate::ast::utils;
use crate::error::ParseError;
use crate::parser::Parser;
use crate::token::{Kind, Token};
use crate::traits::{Parse, Resolve};
use runestick::unit::Span;
use runestick::Source;
use std::borrow::Cow;

/// A string literal.
#[derive(Debug, Clone)]
pub struct LitStr {
    /// The token corresponding to the literal.
    token: Token,
    /// If the string literal is escaped.
    escaped: bool,
}

impl LitStr {
    /// Access the span of the expression.
    pub fn span(&self) -> Span {
        self.token.span
    }
}

impl LitStr {
    fn parse_escaped(&self, span: Span, source: &str) -> Result<String, ParseError> {
        let mut buffer = String::with_capacity(source.len());
        let mut it = source
            .char_indices()
            .map(|(n, c)| (span.start + n, c))
            .peekable();

        while let Some((n, c)) = it.next() {
            buffer.push(match c {
                '\\' => {
                    utils::parse_char_escape(span.with_start(n), &mut it, utils::WithBrace(false))?
                }
                c => c,
            });
        }

        Ok(buffer)
    }
}

impl<'a> Resolve<'a> for LitStr {
    type Output = Cow<'a, str>;

    fn resolve(&self, source: &'a Source) -> Result<Cow<'a, str>, ParseError> {
        let span = self.token.span.narrow(1);
        let string = source
            .source(span)
            .ok_or_else(|| ParseError::BadSlice { span })?;

        Ok(if self.escaped {
            Cow::Owned(self.parse_escaped(span, string)?)
        } else {
            Cow::Borrowed(string)
        })
    }
}

/// Parse a string literal.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// let item = parse_all::<ast::LitStr>("\"hello world\"").unwrap();
/// let item = parse_all::<ast::LitStr>("\"hello\\nworld\"").unwrap();
/// ```
impl Parse for LitStr {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let token = parser.token_next()?;

        match token.kind {
            Kind::LitStr { escaped } => Ok(LitStr { token, escaped }),
            _ => Err(ParseError::ExpectedString {
                actual: token.kind,
                span: token.span,
            }),
        }
    }
}
