use crate::ast;
use crate::{IntoTokens, Parse, ParseError, ParseErrorKind, Parser, Resolve, Spanned, Storage};
use runestick::{Source, Span};

/// A byte literal.
#[derive(Debug, Clone)]
pub struct LitByte {
    /// The token corresponding to the literal.
    pub token: ast::Token,
    /// The source of the byte.
    pub source: ast::CopySource<u8>,
}

impl Spanned for LitByte {
    fn span(&self) -> Span {
        self.token.span()
    }
}

/// Parse a byte literal.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// parse_all::<ast::LitByte>("b'a'").unwrap();
/// parse_all::<ast::LitByte>("b'\\0'").unwrap();
/// parse_all::<ast::LitByte>("b'\\n'").unwrap();
/// parse_all::<ast::LitByte>("b'\\r'").unwrap();
/// parse_all::<ast::LitByte>("b'\\\\''").unwrap();
/// ```
impl Parse for LitByte {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let token = parser.token_next()?;

        Ok(match token.kind {
            ast::Kind::LitByte(source) => LitByte { token, source },
            _ => {
                return Err(ParseError::new(
                    token,
                    ParseErrorKind::ExpectedByte { actual: token.kind },
                ));
            }
        })
    }
}

impl<'a> Resolve<'a> for LitByte {
    type Output = u8;

    fn resolve(&self, _: &Storage, source: &'a Source) -> Result<u8, ParseError> {
        match self.source {
            ast::CopySource::Inline(b) => return Ok(b),
            ast::CopySource::Text => (),
        }

        let span = self.token.span();

        let string = source
            .source(span.trim_start(2).trim_end(1))
            .ok_or_else(|| ParseError::new(span, ParseErrorKind::BadSlice))?;

        let mut it = string
            .char_indices()
            .map(|(n, c)| (span.start + n, c))
            .peekable();

        let (n, c) = match it.next() {
            Some(c) => c,
            None => {
                return Err(ParseError::new(span, ParseErrorKind::BadByteLiteral));
            }
        };

        let c = match c {
            '\\' => ast::utils::parse_byte_escape(span.with_start(n), &mut it)?,
            c if c.is_ascii() && !c.is_control() => c as u8,
            _ => {
                return Err(ParseError::new(span, ParseErrorKind::BadByteLiteral));
            }
        };

        // Too many characters in literal.
        if it.next().is_some() {
            return Err(ParseError::new(span, ParseErrorKind::BadByteLiteral));
        }

        Ok(c)
    }
}

impl IntoTokens for LitByte {
    fn into_tokens(&self, context: &mut crate::MacroContext, stream: &mut crate::TokenStream) {
        self.token.into_tokens(context, stream);
    }
}
