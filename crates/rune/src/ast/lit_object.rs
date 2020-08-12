use crate::ast;
use crate::error::{ParseError, Result};
use crate::parser::Parser;
use crate::source::Source;
use crate::token::Kind;
use crate::traits::{Parse, Resolve};
use runestick::unit::Span;
use std::borrow::Cow;

/// A literal object identifier.
#[derive(Debug, Clone)]
pub enum LitObjectIdent {
    /// An anonymous object.
    Anonymous(ast::Hash),
    /// A named object.
    Named(ast::Path),
}

impl LitObjectIdent {
    /// Get the span of the identifier.
    pub fn span(&self) -> Span {
        match self {
            Self::Anonymous(hash) => hash.span(),
            Self::Named(path) => path.span(),
        }
    }
}

impl Parse for LitObjectIdent {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let token = parser.token_peek_eof()?;

        Ok(match token.kind {
            Kind::Hash => Self::Anonymous(parser.parse()?),
            _ => Self::Named(parser.parse()?),
        })
    }
}

/// Possible literal object keys.
#[derive(Debug, Clone)]
pub enum LitObjectKey {
    /// A literal string (with escapes).
    LitStr(ast::LitStr),
    /// An identifier.
    Ident(ast::Ident),
}

impl LitObjectKey {
    /// Get the span of the object key.
    pub fn span(&self) -> Span {
        match self {
            Self::LitStr(lit_str) => lit_str.span(),
            Self::Ident(ident) => ident.span(),
        }
    }
}

/// Parse an object literal.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// # fn main() -> rune::Result<()> {
/// parse_all::<ast::LitObjectKey>("foo")?;
/// parse_all::<ast::LitObjectKey>("\"foo \\n bar\"")?;
/// # Ok(())
/// # }
/// ```
impl Parse for LitObjectKey {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let token = parser.token_peek_eof()?;

        Ok(match token.kind {
            Kind::LitStr { .. } => Self::LitStr(parser.parse()?),
            Kind::Ident => Self::Ident(parser.parse()?),
            _ => {
                return Err(ParseError::ExpectedLitObjectKey {
                    actual: token.kind,
                    span: token.span,
                })
            }
        })
    }
}

impl<'a> Resolve<'a> for LitObjectKey {
    type Output = Cow<'a, str>;

    fn resolve(&self, source: Source<'a>) -> Result<Self::Output, ParseError> {
        Ok(match self {
            Self::LitStr(lit_str) => lit_str.resolve(source)?,
            Self::Ident(ident) => Cow::Borrowed(ident.resolve(source)?),
        })
    }
}

/// A number literal.
#[derive(Debug, Clone)]
pub struct LitObject {
    /// An object identifier.
    pub ident: LitObjectIdent,
    /// The open bracket.
    pub open: ast::OpenBrace,
    /// Items in the object declaration.
    pub items: Vec<(LitObjectKey, ast::Colon, ast::Expr)>,
    /// The close bracket.
    pub close: ast::CloseBrace,
    /// Indicates if the object is completely literal and cannot have side
    /// effects.
    is_const: bool,
}

impl LitObject {
    /// Access the span of the expression.
    pub fn span(&self) -> Span {
        self.ident.span().join(self.close.span())
    }

    /// Test if the entire expression is constant.
    pub fn is_const(&self) -> bool {
        self.is_const
    }

    /// Parse a literal object with the given path.
    pub fn parse_with_ident(
        parser: &mut Parser<'_>,
        ident: ast::LitObjectIdent,
    ) -> Result<Self, ParseError> {
        let open = parser.parse()?;

        let mut items = Vec::new();

        let mut is_const = true;

        while !parser.peek::<ast::CloseBrace>()? {
            let key = parser.parse()?;
            let colon = parser.parse()?;
            let expr = parser.parse::<ast::Expr>()?;

            if !expr.is_const() {
                is_const = false;
            }

            items.push((key, colon, expr));

            if parser.peek::<ast::Comma>()? {
                parser.parse::<ast::Comma>()?;
            } else {
                break;
            }
        }

        let close = parser.parse()?;

        Ok(Self {
            ident,
            open,
            items,
            close,
            is_const,
        })
    }
}

/// Parse an object literal.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// parse_all::<ast::LitObject>("Foo {\"foo\": 42}").unwrap();
/// parse_all::<ast::LitObject>("#{\"foo\": 42}").unwrap();
/// parse_all::<ast::LitObject>("#{\"foo\": 42,}").unwrap();
/// ```
impl Parse for LitObject {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let ident = parser.parse()?;
        Self::parse_with_ident(parser, ident)
    }
}
