use crate::error::{ParseError, ResolveError, Result};
use crate::parser::Parser;
use crate::source::Source;
use crate::token::{Kind, Token};
use crate::traits::{Parse, Resolve};
use st::unit::Span;
use std::borrow::Cow;

/// A string literal.
#[derive(Debug, Clone)]
pub struct LitStr {
    /// The token corresponding to the literal.
    token: Token,
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
    fn parse_escaped(&self, source: &str) -> Result<String, ResolveError> {
        let mut buffer = String::with_capacity(source.len());
        let mut it = source.chars();

        while let Some(c) = it.next() {
            match (c, it.clone().next()) {
                ('\\', Some('0')) => {
                    buffer.push('\0');
                    it.next();
                }
                ('\\', Some('n')) => {
                    buffer.push('\n');
                    it.next();
                }
                ('\\', Some('r')) => {
                    buffer.push('\r');
                    it.next();
                }
                ('\\', Some('"')) => {
                    buffer.push('"');
                    it.next();
                }
                ('\\', other) => {
                    return Err(ResolveError::BadStringEscapeSequence {
                        c: other.unwrap_or_default(),
                        span: self.token.span,
                    })
                }
                (c, _) => {
                    buffer.push(c);
                }
            }
        }

        Ok(buffer)
    }
}

impl<'a> Resolve<'a> for LitStr {
    type Output = Cow<'a, str>;

    fn resolve(&self, source: Source<'a>) -> Result<Cow<'a, str>, ResolveError> {
        let string = source.source(self.token.span.narrow(1))?;

        Ok(if self.escaped {
            Cow::Owned(self.parse_escaped(string)?)
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
/// use rune::{ParseAll, parse_all, ast, Resolve as _};
///
/// # fn main() -> anyhow::Result<()> {
/// let ParseAll { source, item } = parse_all::<ast::LitStr>("\"hello world\"")?;
/// assert_eq!(item.resolve(source)?, "hello world");
///
/// let ParseAll { source, item } = parse_all::<ast::LitStr>("\"hello\\nworld\"")?;
/// assert_eq!(item.resolve(source)?, "hello\nworld");
/// # Ok(())
/// # }
/// ```
impl Parse for LitStr {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let token = parser.token_next()?;

        match token.kind {
            Kind::LitStr { escaped } => Ok(LitStr { token, escaped }),
            _ => Err(ParseError::ExpectedStringError {
                actual: token.kind,
                span: token.span,
            }),
        }
    }
}
