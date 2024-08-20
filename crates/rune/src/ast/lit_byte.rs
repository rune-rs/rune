use crate::ast::prelude::*;

#[test]
fn ast_parse() {
    rt::<ast::LitByte>("b'a'");
    rt::<ast::LitByte>("b'\\0'");
    rt::<ast::LitByte>("b'\\n'");
    rt::<ast::LitByte>("b'\\r'");
    rt::<ast::LitByte>("b'\\\\''");
}

/// A byte literal.
#[derive(Debug, TryClone, Clone, Copy, PartialEq, Eq, Spanned)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct LitByte {
    /// The span corresponding to the literal.
    pub span: Span,
    /// The source of the byte.
    #[rune(skip)]
    pub source: ast::CopySource<u8>,
}

impl ToAst for LitByte {
    fn to_ast(span: Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            K![byte(source)] => Ok(LitByte { span, source }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                Self::into_expectation(),
            )),
        }
    }

    #[inline]
    fn matches(kind: &ast::Kind) -> bool {
        matches!(kind, K![byte])
    }

    #[inline]
    fn into_expectation() -> Expectation {
        Expectation::Description("byte literal")
    }
}

impl Parse for LitByte {
    fn parse(parser: &mut Parser<'_>) -> Result<Self> {
        let t = parser.next()?;
        Self::to_ast(t.span, t.kind)
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
            .ok_or_else(|| compile::Error::new(span, ErrorKind::BadSlice))?;

        let start = span.start.into_usize();

        let mut it = string
            .char_indices()
            .map(|(n, c)| (start + n, c))
            .peekable();

        let (start, c) = match it.next() {
            Some(c) => c,
            None => {
                return Err(compile::Error::new(span, ErrorKind::BadByteLiteral));
            }
        };

        let c = match c {
            '\\' => {
                let c = match ast::unescape::parse_byte_escape(
                    &mut it,
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
                            ErrorKind::BadByteLiteral,
                        ));
                    }
                }
            }
            c if c.is_ascii() && !c.is_control() => c as u8,
            _ => {
                return Err(compile::Error::new(span, ErrorKind::BadByteLiteral));
            }
        };

        // Too many characters in literal.
        if it.next().is_some() {
            return Err(compile::Error::new(span, ErrorKind::BadByteLiteral));
        }

        Ok(c)
    }
}

impl ToTokens for LitByte {
    fn to_tokens(
        &self,
        _: &mut MacroContext<'_, '_, '_>,
        stream: &mut TokenStream,
    ) -> alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Byte(self.source),
        })
    }
}
