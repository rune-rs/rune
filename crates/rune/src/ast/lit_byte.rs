use crate::ast::prelude::*;

/// A byte literal.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
#[non_exhaustive]
pub struct LitByte {
    /// The token corresponding to the literal.
    pub token: ast::Token,
    /// The source of the byte.
    #[rune(skip)]
    pub source: ast::CopySource<u8>,
}

/// Parse a byte literal.
///
/// # Examples
///
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::LitByte>("b'a'");
/// testing::roundtrip::<ast::LitByte>("b'\\0'");
/// testing::roundtrip::<ast::LitByte>("b'\\n'");
/// testing::roundtrip::<ast::LitByte>("b'\\r'");
/// testing::roundtrip::<ast::LitByte>("b'\\\\''");
/// ```
impl Parse for LitByte {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let token = parser.next()?;

        match token.kind {
            K![byte(source)] => Ok(LitByte { token, source }),
            _ => Err(ParseError::expected(token, "byte")),
        }
    }
}

impl<'a> Resolve<'a> for LitByte {
    type Output = u8;

    fn resolve(&self, _: &'a Storage, sources: &'a Sources) -> Result<u8, ResolveError> {
        let source_id = match self.source {
            ast::CopySource::Inline(b) => return Ok(b),
            ast::CopySource::Text(source_id) => source_id,
        };

        let span = self.token.span();

        let string = sources
            .source(source_id, span.trim_start(2).trim_end(1))
            .ok_or_else(|| ResolveError::new(span, ResolveErrorKind::BadSlice))?;

        let start = span.start.into_usize();

        let mut it = string
            .char_indices()
            .map(|(n, c)| (start + n, c))
            .peekable();

        let (start, c) = match it.next() {
            Some(c) => c,
            None => {
                return Err(ResolveError::new(span, ResolveErrorKind::BadByteLiteral));
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
                            ResolveErrorKind::BadByteLiteral,
                        ));
                    }
                }
            }
            c if c.is_ascii() && !c.is_control() => c as u8,
            _ => {
                return Err(ResolveError::new(span, ResolveErrorKind::BadByteLiteral));
            }
        };

        // Too many characters in literal.
        if it.next().is_some() {
            return Err(ResolveError::new(span, ResolveErrorKind::BadByteLiteral));
        }

        Ok(c)
    }
}
