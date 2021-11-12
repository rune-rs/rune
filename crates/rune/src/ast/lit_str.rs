use crate::ast::prelude::*;
use std::borrow::Cow;

/// A string literal.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub struct LitStr {
    /// The token corresponding to the literal.
    pub token: ast::Token,
    /// The source of the literal string.
    #[rune(skip)]
    pub source: ast::StrSource,
}

impl LitStr {
    /// Resolve a template string.
    pub(crate) fn resolve_template_string<'a>(
        &self,
        storage: &'a Storage,
        sources: &'a Sources,
    ) -> Result<Cow<'a, str>, ResolveError> {
        self.resolve_string(storage, sources, ast::utils::WithTemplate(true))
    }

    /// Resolve the given string with the specified configuration.
    pub(crate) fn resolve_string<'a>(
        &self,
        storage: &'a Storage,
        sources: &'a Sources,
        with_template: ast::utils::WithTemplate,
    ) -> Result<Cow<'a, str>, ResolveError> {
        let span = self.token.span();

        let text = match self.source {
            ast::StrSource::Text(text) => text,
            ast::StrSource::Synthetic(id) => {
                let bytes = storage.get_string(id).ok_or_else(|| {
                    ResolveError::new(
                        span,
                        ResolveErrorKind::BadSyntheticId { kind: "string", id },
                    )
                })?;

                return Ok(Cow::Borrowed(bytes));
            }
        };

        let span = if text.wrapped { span.narrow(1) } else { span };

        let string = sources
            .source(text.source_id, span)
            .ok_or_else(|| ResolveError::new(span, ResolveErrorKind::BadSlice))?;

        Ok(if text.escaped {
            Cow::Owned(Self::parse_escaped(span, string, with_template)?)
        } else {
            Cow::Borrowed(string)
        })
    }

    fn parse_escaped(
        span: Span,
        source: &str,
        with_template: ast::utils::WithTemplate,
    ) -> Result<String, ResolveError> {
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
                    with_template,
                    ast::utils::WithLineCont(true),
                ) {
                    Ok(c) => c,
                    Err(kind) => {
                        let end = it
                            .next()
                            .map(|n| n.0)
                            .unwrap_or_else(|| span.end.into_usize());
                        return Err(ResolveError::new(Span::new(start, end), kind));
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
        let token = parser.next()?;

        match token.kind {
            K![str(source)] => Ok(Self { token, source }),
            _ => Err(ParseError::expected(&token, "string literal")),
        }
    }
}

impl<'a> Resolve<'a> for LitStr {
    type Output = Cow<'a, str>;

    fn resolve(
        &self,
        storage: &'a Storage,
        sources: &'a Sources,
    ) -> Result<Cow<'a, str>, ResolveError> {
        self.resolve_string(storage, sources, ast::utils::WithTemplate(false))
    }
}
