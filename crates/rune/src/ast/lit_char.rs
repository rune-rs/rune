use crate::ast;
use crate::{
    Parse, ParseError, ParseErrorKind, Parser, Resolve, ResolveOwned, Spanned, Storage, ToTokens,
};
use runestick::{Source, Span};

/// A character literal.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub struct LitChar {
    /// The token corresponding to the literal.
    pub token: ast::Token,
    /// The source of the literal character.
    #[rune(skip)]
    pub source: ast::CopySource<char>,
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
        let token = parser.token_next()?;

        match token.kind {
            ast::Kind::LitChar(source) => Ok(LitChar { token, source }),
            _ => Err(ParseError::expected(token, "char")),
        }
    }
}

impl<'a> Resolve<'a> for LitChar {
    type Output = char;

    fn resolve(&self, _: &Storage, source: &'a Source) -> Result<char, ParseError> {
        match self.source {
            ast::CopySource::Inline(c) => return Ok(c),
            ast::CopySource::Text => (),
        }

        let span = self.token.span();

        let string = source
            .source(span.narrow(1))
            .ok_or_else(|| ParseError::new(span, ParseErrorKind::BadSlice))?;

        let mut it = string
            .char_indices()
            .map(|(n, c)| (span.start + n, c))
            .peekable();

        let (start, c) = match it.next() {
            Some(c) => c,
            None => {
                return Err(ParseError::new(span, ParseErrorKind::BadCharLiteral));
            }
        };

        let c = match c {
            '\\' => {
                let c = match ast::utils::parse_char_escape(
                    &mut it,
                    ast::utils::WithBrace(false),
                    ast::utils::WithLineCont(false),
                ) {
                    Ok(c) => c,
                    Err(kind) => {
                        let end = it.next().map(|n| n.0).unwrap_or(span.end);
                        return Err(ParseError::new(Span::new(start, end), kind));
                    }
                };

                match c {
                    Some(c) => c,
                    None => {
                        let end = it.next().map(|n| n.0).unwrap_or(span.end);
                        return Err(ParseError::new(
                            Span::new(start, end),
                            ParseErrorKind::BadCharLiteral,
                        ));
                    }
                }
            }
            c => c,
        };

        // Too many characters in literal.
        if it.next().is_some() {
            return Err(ParseError::new(span, ParseErrorKind::BadCharLiteral));
        }

        Ok(c)
    }
}

impl ResolveOwned for LitChar {
    type Owned = char;

    fn resolve_owned(&self, storage: &Storage, source: &Source) -> Result<Self::Owned, ParseError> {
        Ok(self.resolve(storage, source)?)
    }
}
