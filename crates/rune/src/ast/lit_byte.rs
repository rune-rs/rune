use crate::ast;
use crate::{Parse, ParseError, ParseErrorKind, Parser, Resolve, Spanned, Storage, ToTokens};
use runestick::Source;

/// A byte literal.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
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
        let token = parser.token_next()?;

        Ok(match token.kind {
            ast::Kind::LitByte(source) => LitByte { token, source },
            _ => {
                return Err(ParseError::expected(token, "byte"));
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
