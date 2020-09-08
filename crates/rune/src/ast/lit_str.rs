use crate::ast;
use crate::{IntoTokens, Parse, ParseError, Parser, Resolve, Storage};
use runestick::{Source, Span};
use std::borrow::Cow;

/// A string literal.
#[derive(Debug, Clone)]
pub struct LitStr {
    /// The token corresponding to the literal.
    token: ast::Token,
    /// If the string literal is escaped.
    escaped: bool,
}

impl LitStr {
    /// Access the span of the expression.
    pub fn span(&self) -> Span {
        self.token.span
    }
}

impl LitStr {
    fn parse_escaped(&self, span: Span, source: &str) -> Result<String, ParseError> {
        let mut buffer = String::with_capacity(source.len());
        let mut it = source
            .char_indices()
            .map(|(n, c)| (span.start + n, c))
            .peekable();

        while let Some((n, c)) = it.next() {
            buffer.push(match c {
                '\\' => ast::utils::parse_char_escape(
                    span.with_start(n),
                    &mut it,
                    ast::utils::WithBrace(false),
                )?,
                c => c,
            });
        }

        Ok(buffer)
    }
}

impl<'a> Resolve<'a> for LitStr {
    type Output = Cow<'a, str>;

    fn resolve(&self, _: &Storage, source: &'a Source) -> Result<Cow<'a, str>, ParseError> {
        let span = self.token.span.narrow(1);

        let string = source
            .source(span)
            .ok_or_else(|| ParseError::BadSlice { span })?;

        Ok(if self.escaped {
            Cow::Owned(self.parse_escaped(span, string)?)
        } else {
            Cow::Borrowed(string)
        })
    }
}

/// Parse a string literal.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// let item = parse_all::<ast::LitStr>("\"hello world\"").unwrap();
/// let item = parse_all::<ast::LitStr>("\"hello\\nworld\"").unwrap();
/// ```
impl Parse for LitStr {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let token = parser.token_next()?;

        match token.kind {
            ast::Kind::LitStr { escaped } => Ok(LitStr { token, escaped }),
            _ => Err(ParseError::ExpectedString {
                actual: token.kind,
                span: token.span,
            }),
        }
    }
}

impl IntoTokens for LitStr {
    fn into_tokens(&self, _: &mut crate::MacroContext, stream: &mut crate::TokenStream) {
        stream.push(self.token);
    }
}
