use crate::ast::prelude::*;

/// A character literal.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub struct LitChar {
    /// The token corresponding to the literal.
    pub token: ast::Token,
    /// The source of the literal character.
    #[rune(skip)]
    pub source: ast::CopySource<char>,
}

impl LitChar {
    /// Construct a new literal character.
    pub fn new(ctx: &mut MacroContext<'_, '_>, c: char) -> Self {
        Self {
            token: ast::Token {
                kind: ast::Kind::Char(ast::CopySource::Inline(c)),
                span: ctx.macro_span(),
            },
            source: ast::CopySource::Inline(c),
        }
    }
}

/// Parse a character literal.
///
/// # Examples
///
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::LitChar>("'a'");
/// testing::roundtrip::<ast::LitChar>("'\\0'");
/// testing::roundtrip::<ast::LitChar>("'\\n'");
/// testing::roundtrip::<ast::LitChar>("'\\r'");
/// testing::roundtrip::<ast::LitChar>("'\\''");
/// ```
impl Parse for LitChar {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let token = parser.next()?;

        match token.kind {
            K![char(source)] => Ok(LitChar { token, source }),
            _ => Err(ParseError::expected(&token, "char")),
        }
    }
}

impl<'a> Resolve<'a> for LitChar {
    type Output = char;

    fn resolve(&self, _: &'a Storage, sources: &'a Sources) -> Result<char, ResolveError> {
        let source_id = match self.source {
            ast::CopySource::Inline(c) => return Ok(c),
            ast::CopySource::Text(source_id) => source_id,
        };

        let span = self.token.span();

        let string = sources
            .source(source_id, span.narrow(1))
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

impl ResolveOwned for LitChar {
    type Owned = char;

    fn resolve_owned(
        &self,
        storage: &Storage,
        sources: &Sources,
    ) -> Result<Self::Owned, ResolveError> {
        self.resolve(storage, sources)
    }
}
