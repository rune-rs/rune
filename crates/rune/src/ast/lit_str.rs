use crate::alloc::borrow::Cow;
use crate::ast::prelude::*;

#[test]
fn ast_parse() {
    use crate::testing::rt;

    rt::<ast::LitStr>("\"hello world\"");
    rt::<ast::LitStr>("\"hello\\nworld\"");
}

/// A string literal.
///
/// * `"Hello World"`.
/// * `"Hello\nWorld"`.
#[derive(Debug, TryClone, Clone, Copy, PartialEq, Eq, Spanned)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct LitStr {
    /// The span corresponding to the literal.
    pub span: Span,
    /// The source of the literal string.
    #[rune(skip)]
    pub source: ast::StrSource,
}

impl LitStr {
    /// Resolve a template string.
    pub(crate) fn resolve_template_string<'a>(
        &self,
        cx: ResolveContext<'a>,
    ) -> Result<Cow<'a, str>> {
        self.resolve_inner(cx, ast::unescape::WithTemplate(true))
    }

    /// Resolve as a regular string.
    pub(crate) fn resolve_string<'a>(&self, cx: ResolveContext<'a>) -> Result<Cow<'a, str>> {
        self.resolve_inner(cx, ast::unescape::WithTemplate(false))
    }

    /// Resolve the given string with the specified configuration.
    fn resolve_inner<'a>(
        &self,
        cx: ResolveContext<'a>,
        with_template: ast::unescape::WithTemplate,
    ) -> Result<Cow<'a, str>> {
        let span = self.span;

        let text = match self.source {
            ast::StrSource::Text(text) => text,
            ast::StrSource::Synthetic(id) => {
                let bytes = cx.storage.get_string(id).ok_or_else(|| {
                    compile::Error::new(
                        span,
                        ErrorKind::BadSyntheticId {
                            kind: SyntheticKind::String,
                            id,
                        },
                    )
                })?;

                return Ok(Cow::Borrowed(bytes));
            }
        };

        let span = if text.wrapped {
            span.narrow(1u32)
        } else {
            span
        };

        let string = cx
            .sources
            .source(text.source_id, span)
            .ok_or_else(|| compile::Error::new(span, ErrorKind::BadSlice))?;

        Ok(if text.escaped {
            Cow::Owned(Self::parse_escaped(span, string, with_template)?)
        } else {
            Cow::Borrowed(string)
        })
    }

    fn parse_escaped(
        span: Span,
        source: &str,
        with_template: ast::unescape::WithTemplate,
    ) -> Result<String> {
        let mut buffer = String::try_with_capacity(source.len())?;

        let start = span.start.into_usize();

        let mut it = source
            .char_indices()
            .map(|(n, c)| (start + n, c))
            .peekable();

        while let Some((start, c)) = it.next() {
            buffer.try_extend(match c {
                '\\' => match ast::unescape::parse_char_escape(
                    &mut it,
                    with_template,
                    ast::unescape::WithLineCont(true),
                ) {
                    Ok(c) => c,
                    Err(kind) => {
                        let end = it
                            .next()
                            .map(|n| n.0)
                            .unwrap_or_else(|| span.end.into_usize());
                        return Err(compile::Error::new(Span::new(start, end), kind));
                    }
                },
                c => Some(c),
            })?;
        }

        Ok(buffer)
    }
}

impl Parse for LitStr {
    fn parse(parser: &mut Parser<'_>) -> Result<Self> {
        let t = parser.next()?;

        match t.kind {
            K![str(source)] => Ok(Self {
                span: t.span,
                source,
            }),
            _ => Err(compile::Error::expected(t, "string literal")),
        }
    }
}

impl<'a> Resolve<'a> for LitStr {
    type Output = Cow<'a, str>;

    fn resolve(&self, cx: ResolveContext<'a>) -> Result<Cow<'a, str>> {
        self.resolve_string(cx)
    }
}

impl ToTokens for LitStr {
    fn to_tokens(
        &self,
        _: &mut MacroContext<'_, '_, '_>,
        stream: &mut TokenStream,
    ) -> alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Str(self.source),
        })
    }
}
