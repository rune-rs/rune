use crate::ast;
use crate::{IntoTokens, Parse, ParseError, Parser, Resolve, Storage};
use runestick::{Source, Span};

/// A character literal.
#[derive(Debug, Clone)]
pub struct LitChar {
    /// The token corresponding to the literal.
    pub token: ast::Token,
    /// The source of the literal character.
    pub source: ast::CopySource<char>,
}

impl LitChar {
    /// Access the span of the expression.
    pub fn span(&self) -> Span {
        self.token.span
    }
}

/// Parse a character literal.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// parse_all::<ast::LitChar>("'a'").unwrap();
/// parse_all::<ast::LitChar>("'\\0'").unwrap();
/// parse_all::<ast::LitChar>("'\\n'").unwrap();
/// parse_all::<ast::LitChar>("'\\r'").unwrap();
/// parse_all::<ast::LitChar>("'\\''").unwrap();
/// ```
impl Parse for LitChar {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let token = parser.token_next()?;

        Ok(match token.kind {
            ast::Kind::LitChar(source) => LitChar { token, source },
            _ => {
                return Err(ParseError::ExpectedChar {
                    actual: token.kind,
                    span: token.span,
                })
            }
        })
    }
}

impl<'a> Resolve<'a> for LitChar {
    type Output = char;

    fn resolve(&self, _: &Storage, source: &'a Source) -> Result<char, ParseError> {
        match self.source {
            ast::CopySource::Inline(c) => return Ok(c),
            ast::CopySource::Text => (),
        }

        let span = self.token.span;
        let string = source
            .source(span.narrow(1))
            .ok_or_else(|| ParseError::BadSlice { span })?;
        let mut it = string
            .char_indices()
            .map(|(n, c)| (span.start + n, c))
            .peekable();

        let (n, c) = match it.next() {
            Some(c) => c,
            None => {
                return Err(ParseError::BadCharLiteral { span });
            }
        };

        let c = match c {
            '\\' => ast::utils::parse_char_escape(
                span.with_start(n),
                &mut it,
                ast::utils::WithBrace(false),
            )?,
            c => c,
        };

        // Too many characters in literal.
        if it.next().is_some() {
            return Err(ParseError::BadCharLiteral { span });
        }

        Ok(c)
    }
}

impl IntoTokens for LitChar {
    fn into_tokens(&self, context: &mut crate::MacroContext, stream: &mut crate::TokenStream) {
        self.token.into_tokens(context, stream);
    }
}
