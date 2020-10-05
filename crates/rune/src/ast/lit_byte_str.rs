use crate::ast;
use crate::{
    Parse, ParseError, ParseErrorKind, Parser, Resolve, ResolveOwned, Spanned, Storage, ToTokens,
};
use runestick::{Source, Span};
use std::borrow::Cow;

/// A string literal.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub struct LitByteStr {
    /// The token corresponding to the literal.
    pub token: ast::Token,
    /// If the string literal is escaped.
    #[rune(skip)]
    pub source: ast::LitStrSource,
}

impl LitByteStr {
    fn parse_escaped(&self, span: Span, source: &str) -> Result<Vec<u8>, ParseError> {
        let mut buffer = Vec::with_capacity(source.len());

        let mut it = source
            .char_indices()
            .map(|(n, c)| (span.start + n, c))
            .peekable();

        while let Some((start, c)) = it.next() {
            buffer.extend(match c {
                '\\' => {
                    match ast::utils::parse_byte_escape(&mut it, ast::utils::WithLineCont(true)) {
                        Ok(c) => c,
                        Err(kind) => {
                            let end = it.next().map(|n| n.0).unwrap_or(span.end);
                            return Err(ParseError::new(Span::new(start, end), kind));
                        }
                    }
                }
                c => Some(c as u8),
            });
        }

        Ok(buffer)
    }
}

/// Parse a string literal.
///
/// # Examples
///
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::LitByteStr>("b\"hello world\"");
/// testing::roundtrip::<ast::LitByteStr>("b\"hello\\nworld\"");
/// ```
impl Parse for LitByteStr {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let token = parser.token_next()?;

        match token.kind {
            ast::Kind::LitByteStr(source) => Ok(Self { token, source }),
            _ => Err(ParseError::expected(token, "literal byte string")),
        }
    }
}

impl<'a> Resolve<'a> for LitByteStr {
    type Output = Cow<'a, [u8]>;

    fn resolve(&self, storage: &Storage, source: &'a Source) -> Result<Cow<'a, [u8]>, ParseError> {
        let span = self.token.span();

        let text = match self.source {
            ast::LitStrSource::Text(text) => text,
            ast::LitStrSource::Synthetic(id) => {
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

impl ResolveOwned for LitByteStr {
    type Owned = Vec<u8>;

    fn resolve_owned(&self, storage: &Storage, source: &Source) -> Result<Self::Owned, ParseError> {
        Ok(self.resolve(storage, source)?.into_owned())
    }
}
