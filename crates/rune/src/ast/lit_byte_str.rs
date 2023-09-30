use crate::alloc::borrow::Cow;
use crate::ast::prelude::*;

#[test]
fn ast_parse() {
    use crate::testing::rt;

    rt::<ast::LitByteStr>("b\"hello world\"");
    rt::<ast::LitByteStr>("b\"hello\\nworld\"");
}

/// A string literal.
///
/// * `"Hello World"`.
#[derive(Debug, TryClone, Clone, Copy, PartialEq, Eq, Spanned)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct LitByteStr {
    /// The span corresponding to the literal.
    pub span: Span,
    /// If the string literal is escaped.
    #[rune(skip)]
    pub source: ast::StrSource,
}

impl LitByteStr {
    fn parse_escaped(&self, span: Span, source: &str) -> Result<Vec<u8>> {
        let mut buffer = Vec::try_with_capacity(source.len())?;

        let start = span.start.into_usize();

        let mut it = source
            .char_indices()
            .map(|(n, c)| (start + n, c))
            .peekable();

        while let Some((start, c)) = it.next() {
            buffer.try_extend(match c {
                '\\' => {
                    match ast::unescape::parse_byte_escape(
                        &mut it,
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
                    }
                }
                c => Some(c as u8),
            })?;
        }

        Ok(buffer)
    }
}

impl Parse for LitByteStr {
    fn parse(parser: &mut Parser<'_>) -> Result<Self> {
        let t = parser.next()?;

        match t.kind {
            K![bytestr(source)] => Ok(Self {
                span: t.span,
                source,
            }),
            _ => Err(compile::Error::expected(t, "byte string")),
        }
    }
}

impl<'a> Resolve<'a> for LitByteStr {
    type Output = Cow<'a, [u8]>;

    fn resolve(&self, cx: ResolveContext<'a>) -> Result<Cow<'a, [u8]>> {
        let span = self.span;

        let text = match self.source {
            ast::StrSource::Text(text) => text,
            ast::StrSource::Synthetic(id) => {
                let bytes = cx.storage.get_byte_string(id).ok_or_else(|| {
                    compile::Error::new(
                        span,
                        ErrorKind::BadSyntheticId {
                            kind: SyntheticKind::ByteString,
                            id,
                        },
                    )
                })?;

                return Ok(Cow::Borrowed(bytes));
            }
        };

        let span = span.trim_start(2u32).trim_end(1u32);
        let string = cx
            .sources
            .source(text.source_id, span)
            .ok_or_else(|| compile::Error::new(span, ErrorKind::BadSlice))?;

        Ok(if text.escaped {
            Cow::Owned(self.parse_escaped(span, string)?)
        } else {
            Cow::Borrowed(string.as_bytes())
        })
    }
}

impl ToTokens for LitByteStr {
    fn to_tokens(
        &self,
        _: &mut MacroContext<'_, '_, '_>,
        stream: &mut TokenStream,
    ) -> alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::ByteStr(self.source),
        })
    }
}
