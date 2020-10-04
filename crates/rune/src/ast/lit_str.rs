use crate::ast;
use crate::{
    Parse, ParseError, ParseErrorKind, Parser, Resolve, ResolveOwned, Spanned, Storage, ToTokens,
};
use runestick::{Source, Span};
use std::borrow::Cow;

/// A string literal.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub struct LitStr {
    /// The token corresponding to the literal.
    pub token: ast::Token,
    /// The source of the literal string.
    #[rune(skip)]
    pub source: ast::LitStrSource,
}

impl LitStr {
    fn parse_escaped(&self, span: Span, source: &str) -> Result<String, ParseError> {
        let mut buffer = String::with_capacity(source.len());

        let mut it = source
            .char_indices()
            .map(|(n, c)| (span.start + n, c))
            .peekable();

        while let Some((n, c)) = it.next() {
            buffer.extend(match c {
                '\\' => ast::utils::parse_char_escape(
                    span.with_start(n),
                    &mut it,
                    ast::utils::WithBrace(false),
                    ast::utils::WithLineCont(true),
                )?,
                c => Some(c),
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
/// testing::roundtrip::<ast::LitStr>("\"hello world\"");
/// testing::roundtrip::<ast::LitStr>("\"hello\\nworld\"");
/// ```
impl Parse for LitStr {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let token = parser.token_next()?;

        match token.kind {
            ast::Kind::LitStr(source) => Ok(Self { token, source }),
            _ => Err(ParseError::expected(token, "string literal")),
        }
    }
}

impl<'a> Resolve<'a> for LitStr {
    type Output = Cow<'a, str>;

    fn resolve(&self, storage: &Storage, source: &'a Source) -> Result<Cow<'a, str>, ParseError> {
        let span = self.token.span();

        let text = match self.source {
            ast::LitStrSource::Text(text) => text,
            ast::LitStrSource::Synthetic(id) => {
                let bytes = storage.get_string(id).ok_or_else(|| {
                    ParseError::new(span, ParseErrorKind::BadSyntheticId { kind: "string", id })
                })?;

                return Ok(Cow::Owned(bytes));
            }
        };

        let span = span.narrow(1);

        let string = source
            .source(span)
            .ok_or_else(|| ParseError::new(span, ParseErrorKind::BadSlice))?;

        Ok(if text.escaped {
            Cow::Owned(self.parse_escaped(span, string)?)
        } else {
            Cow::Borrowed(string)
        })
    }
}

impl ResolveOwned for LitStr {
    type Owned = String;

    fn resolve_owned(&self, storage: &Storage, source: &Source) -> Result<Self::Owned, ParseError> {
        Ok(self.resolve(storage, source)?.into_owned())
    }
}
