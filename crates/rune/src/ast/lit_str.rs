use crate::no_std::borrow::Cow;

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Spanned)]
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
        ctx: ResolveContext<'a>,
    ) -> Result<Cow<'a, str>> {
        self.resolve_string(ctx, ast::utils::WithTemplate(true))
    }

    /// Resolve the given string with the specified configuration.
    pub(crate) fn resolve_string<'a>(
        &self,
        ctx: ResolveContext<'a>,
        with_template: ast::utils::WithTemplate,
    ) -> Result<Cow<'a, str>> {
        let span = self.span;

        let text = match self.source {
            ast::StrSource::Text(text) => text,
            ast::StrSource::Synthetic(id) => {
                let bytes = ctx.storage.get_string(id).ok_or_else(|| {
                    compile::Error::new(
                        span,
                        ResolveErrorKind::BadSyntheticId {
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

        let string = ctx
            .sources
            .source(text.source_id, span)
            .ok_or_else(|| compile::Error::new(span, ResolveErrorKind::BadSlice))?;

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
    ) -> Result<String> {
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
                        return Err(compile::Error::new(Span::new(start, end), kind));
                    }
                },
                c => Some(c),
            });
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

    fn resolve(&self, ctx: ResolveContext<'a>) -> Result<Cow<'a, str>> {
        self.resolve_string(ctx, ast::utils::WithTemplate(false))
    }
}

impl ToTokens for LitStr {
    fn to_tokens(&self, _: &mut MacroContext<'_>, stream: &mut TokenStream) {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Str(self.source),
        });
    }
}
