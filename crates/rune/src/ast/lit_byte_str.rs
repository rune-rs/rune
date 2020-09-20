use crate::ast;
use crate::{Parse, ParseError, ParseErrorKind, Parser, Resolve, Spanned, Storage, ToTokens};
use runestick::{Source, Span};
use std::borrow::Cow;

/// A string literal.
#[derive(Debug, Clone, ToTokens, Spanned)]
pub struct LitByteStr {
    /// The token corresponding to the literal.
    token: ast::Token,
    /// If the string literal is escaped.
    #[rune(skip)]
    source: ast::LitByteStrSource,
}

impl LitByteStr {
    fn parse_escaped(&self, span: Span, source: &str) -> Result<Vec<u8>, ParseError> {
        let mut buffer = Vec::with_capacity(source.len());

        let mut it = source
            .char_indices()
            .map(|(n, c)| (span.start + n, c))
            .peekable();

        while let Some((n, c)) = it.next() {
            buffer.push(match c {
                '\\' => ast::utils::parse_byte_escape(span.with_start(n), &mut it)?,
                c => c as u8,
            });
        }

        Ok(buffer)
    }
}

impl<'a> Resolve<'a> for LitByteStr {
    type Output = Cow<'a, [u8]>;

    fn resolve(&self, storage: &Storage, source: &'a Source) -> Result<Cow<'a, [u8]>, ParseError> {
        let span = self.token.span();

        let text = match self.source {
            ast::LitByteStrSource::Text(text) => text,
            ast::LitByteStrSource::Synthetic(id) => {
                let bytes = storage.get_byte_string(id).ok_or_else(|| {
                    ParseError::new(
                        span,
                        ParseErrorKind::BadSyntheticId {
                            kind: "byte string",
                            id,
                        },
                    )
                })?;

                return Ok(Cow::Owned(bytes));
            }
        };

        let span = span.trim_start(2).trim_end(1);
        let string = source
            .source(span)
            .ok_or_else(|| ParseError::new(span, ParseErrorKind::BadSlice))?;

        Ok(if text.escaped {
            Cow::Owned(self.parse_escaped(span, string)?)
        } else {
            Cow::Borrowed(string.as_bytes())
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
/// let s = parse_all::<ast::LitByteStr>("b\"hello world\"").unwrap();
/// let s = parse_all::<ast::LitByteStr>("b\"hello\\nworld\"").unwrap();
/// ```
impl Parse for LitByteStr {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let token = parser.token_next()?;

        match token.kind {
            ast::Kind::LitByteStr(source) => Ok(Self { token, source }),
            _ => Err(ParseError::new(
                token,
                ParseErrorKind::ExpectedString { actual: token.kind },
            )),
        }
    }
}
