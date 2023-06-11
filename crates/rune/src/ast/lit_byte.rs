use crate::ast::prelude::*;

#[test]
fn ast_parse() {
    use crate::testing::rt;

    rt::<ast::LitByte>("b'a'");
    rt::<ast::LitByte>("b'\\0'");
    rt::<ast::LitByte>("b'\\n'");
    rt::<ast::LitByte>("b'\\r'");
    rt::<ast::LitByte>("b'\\\\''");
}

/// A byte literal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Spanned)]
#[non_exhaustive]
pub struct LitByte {
    /// The span corresponding to the literal.
    pub span: Span,
    /// The source of the byte.
    #[rune(skip)]
    pub source: ast::CopySource<u8>,
}

impl Parse for LitByte {
    fn parse(parser: &mut Parser<'_>) -> Result<Self> {
        let t = parser.next()?;

        match t.kind {
            K![byte(source)] => Ok(LitByte {
                span: t.span,
                source,
            }),
            _ => Err(compile::Error::expected(t, "byte")),
        }
    }
}

impl<'a> Resolve<'a> for LitByte {
    type Output = u8;

    fn resolve(&self, cx: ResolveContext<'a>) -> Result<u8> {
        let source_id = match self.source {
            ast::CopySource::Inline(b) => return Ok(b),
            ast::CopySource::Text(source_id) => source_id,
        };

        let span = self.span;

        let string = cx
            .sources
            .source(source_id, span.trim_start(2u32).trim_end(1u32))
            .ok_or_else(|| compile::Error::new(span, ResolveErrorKind::BadSlice))?;

        let start = span.start.into_usize();

        let mut it = string
            .char_indices()
            .map(|(n, c)| (start + n, c))
            .peekable();

        let (start, c) = match it.next() {
            Some(c) => c,
            None => {
                return Err(compile::Error::new(span, ResolveErrorKind::BadByteLiteral));
            }
        };

        let c = match c {
            '\\' => {
                let c =
                    match ast::utils::parse_byte_escape(&mut it, ast::utils::WithLineCont(false)) {
                        Ok(c) => c,
                        Err(kind) => {
                            let end = it
                                .next()
                                .map(|n| n.0)
                                .unwrap_or_else(|| span.end.into_usize());
                            return Err(compile::Error::new(Span::new(start, end), kind));
                        }
                    };

                match c {
                    Some(c) => c,
                    None => {
                        let end = it
                            .next()
                            .map(|n| n.0)
                            .unwrap_or_else(|| span.end.into_usize());
                        return Err(compile::Error::new(
                            Span::new(start, end),
                            ResolveErrorKind::BadByteLiteral,
                        ));
                    }
                }
            }
            c if c.is_ascii() && !c.is_control() => c as u8,
            _ => {
                return Err(compile::Error::new(span, ResolveErrorKind::BadByteLiteral));
            }
        };

        // Too many characters in literal.
        if it.next().is_some() {
            return Err(compile::Error::new(span, ResolveErrorKind::BadByteLiteral));
        }

        Ok(c)
    }
}

impl ToTokens for LitByte {
    fn to_tokens(&self, _: &mut MacroContext<'_, '_, '_>, stream: &mut TokenStream) {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Byte(self.source),
        });
    }
}
