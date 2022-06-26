use crate::ast::prelude::*;

/// A character literal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Spanned)]
#[non_exhaustive]
pub struct LitChar {
    /// The span corresponding to the literal.
    pub span: Span,
    /// The source of the literal character.
    #[rune(skip)]
    pub source: ast::CopySource<char>,
}

/// Parse a character literal.
///
/// # Examples
///
/// ```
/// use rune::{ast, testing};
///
/// testing::roundtrip::<ast::LitChar>("'a'");
/// testing::roundtrip::<ast::LitChar>("'\\0'");
/// testing::roundtrip::<ast::LitChar>("'\\n'");
/// testing::roundtrip::<ast::LitChar>("'\\r'");
/// testing::roundtrip::<ast::LitChar>("'\\''");
/// ```
impl Parse for LitChar {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let t = parser.next()?;

        match t.kind {
            K![char(source)] => Ok(LitChar {
                span: t.span,
                source,
            }),
            _ => Err(ParseError::expected(t, "char")),
        }
    }
}

impl<'a> Resolve<'a> for LitChar {
    type Output = char;

    fn resolve(&self, ctx: ResolveContext<'a>) -> Result<char, ResolveError> {
        let source_id = match self.source {
            ast::CopySource::Inline(c) => return Ok(c),
            ast::CopySource::Text(source_id) => source_id,
        };

        let span = self.span;

        let string = ctx
            .sources
            .source(source_id, span.narrow(1u32))
            .ok_or_else(|| ResolveError::new(span, ResolveErrorKind::BadSlice))?;

        let start = span.start.into_usize();

        let mut it = string
            .char_indices()
            .map(|(n, c)| (start + n, c))
            .peekable();

        let (start, c) = match it.next() {
            Some(c) => c,
            None => {
                return Err(ResolveError::new(span, ResolveErrorKind::BadCharLiteral));
            }
        };

        let c = match c {
            '\\' => {
                let c = match ast::utils::parse_char_escape(
                    &mut it,
                    ast::utils::WithTemplate(false),
                    ast::utils::WithLineCont(false),
                ) {
                    Ok(c) => c,
                    Err(kind) => {
                        let end = it
                            .next()
                            .map(|n| n.0)
                            .unwrap_or_else(|| span.end.into_usize());
                        return Err(ResolveError::new(Span::new(start, end), kind));
                    }
                };

                match c {
                    Some(c) => c,
                    None => {
                        let end = it
                            .next()
                            .map(|n| n.0)
                            .unwrap_or_else(|| span.end.into_usize());
                        return Err(ResolveError::new(
                            Span::new(start, end),
                            ResolveErrorKind::BadCharLiteral,
                        ));
                    }
                }
            }
            c => c,
        };

        // Too many characters in literal.
        if it.next().is_some() {
            return Err(ResolveError::new(span, ResolveErrorKind::BadCharLiteral));
        }

        Ok(c)
    }
}

impl ToTokens for LitChar {
    fn to_tokens(&self, _: &mut MacroContext<'_>, stream: &mut TokenStream) {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Char(self.source),
        });
    }
}
