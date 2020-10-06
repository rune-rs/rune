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
    /// Resolve a template string.
    pub(crate) fn resolve_template_string<'a>(
        &self,
        storage: &Storage,
        source: &'a Source,
    ) -> Result<Cow<'a, str>, ParseError> {
        self.resolve_string(storage, source, ast::utils::WithBrace(true))
    }

    /// Resolve the given string with the specified configuration.
    pub(crate) fn resolve_string<'a>(
        &self,
        storage: &Storage,
        source: &'a Source,
        with_brace: ast::utils::WithBrace,
    ) -> Result<Cow<'a, str>, ParseError> {
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

        let span = if text.wrapped { span.narrow(1) } else { span };

        let string = source
            .source(span)
            .ok_or_else(|| ParseError::new(span, ParseErrorKind::BadSlice))?;

        Ok(if text.escaped {
            Cow::Owned(Self::parse_escaped(span, string, with_brace)?)
        } else {
            Cow::Borrowed(string)
        })
    }

    fn parse_escaped(
        span: Span,
        source: &str,
        with_brace: ast::utils::WithBrace,
    ) -> Result<String, ParseError> {
        let mut buffer = String::with_capacity(source.len());

        let start = span.start.into_usize();

        let mut it = source
            .char_indices()
            .map(|(n, c)| (start + n, c))
            .peekable();

        while let Some((start, c)) = it.next() {
            buffer.extend(match c {
                '\\' => match ast::utils::parse_char_escape(
                    &mut it,
                    with_brace,
                    ast::utils::WithLineCont(true),
                ) {
                    Ok(c) => c,
                    Err(kind) => {
                        let end = it
                            .next()
                            .map(|n| n.0)
                            .unwrap_or_else(|| span.end.into_usize());
                        return Err(ParseError::new(Span::new(start, end), kind));
                    }
                },
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
        self.resolve_string(storage, source, ast::utils::WithBrace(false))
    }
}

impl ResolveOwned for LitStr {
    type Owned = String;

    fn resolve_owned(&self, storage: &Storage, source: &Source) -> Result<Self::Owned, ParseError> {
        Ok(self.resolve(storage, source)?.into_owned())
    }
}
