use crate::ast::prelude::*;

#[test]
fn ast_parse() {
    use crate::testing::rt;

    rt::<ast::LitChar>("'a'");
    rt::<ast::LitChar>("'\\0'");
    rt::<ast::LitChar>("'\\n'");
    rt::<ast::LitChar>("'\\r'");
    rt::<ast::LitChar>("'\\''");
}

/// A character literal.
#[derive(Debug, TryClone, Clone, Copy, PartialEq, Eq, Spanned)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct LitChar {
    /// The span corresponding to the literal.
    pub span: Span,
    /// The source of the literal character.
    #[rune(skip)]
    pub source: ast::CopySource<char>,
}

impl Parse for LitChar {
    fn parse(parser: &mut Parser<'_>) -> Result<Self> {
        let t = parser.next()?;

        match t.kind {
            K![char(source)] => Ok(LitChar {
                span: t.span,
                source,
            }),
            _ => Err(compile::Error::expected(t, "char")),
        }
    }
}

impl<'a> Resolve<'a> for LitChar {
    type Output = char;

    fn resolve(&self, cx: ResolveContext<'a>) -> Result<char> {
        let source_id = match self.source {
            ast::CopySource::Inline(c) => return Ok(c),
            ast::CopySource::Text(source_id) => source_id,
        };

        let span = self.span;

        let string = cx
            .sources
            .source(source_id, span.narrow(1u32))
            .ok_or_else(|| compile::Error::new(span, ErrorKind::BadSlice))?;

        let start = span.start.into_usize();

        let mut it = string
            .char_indices()
            .map(|(n, c)| (start + n, c))
            .peekable();

        let (start, c) = match it.next() {
            Some(c) => c,
            None => {
                return Err(compile::Error::new(span, ErrorKind::BadCharLiteral));
            }
        };

        let c = match c {
            '\\' => {
                let c = match ast::unescape::parse_char_escape(
                    &mut it,
                    ast::unescape::WithTemplate(false),
                    ast::unescape::WithLineCont(false),
                ) {
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
                            ErrorKind::BadCharLiteral,
                        ));
                    }
                }
            }
            c => c,
        };

        // Too many characters in literal.
        if it.next().is_some() {
            return Err(compile::Error::new(span, ErrorKind::BadCharLiteral));
        }

        Ok(c)
    }
}

impl ToTokens for LitChar {
    fn to_tokens(
        &self,
        _: &mut MacroContext<'_, '_, '_>,
        stream: &mut TokenStream,
    ) -> alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Char(self.source),
        })
    }
}
