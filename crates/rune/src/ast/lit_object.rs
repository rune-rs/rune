use crate::ast;
use crate::{
    Parse, ParseError, ParseErrorKind, Parser, Peek, Resolve, ResolveOwned, Spanned, Storage,
    ToTokens,
};
use runestick::Source;
use std::borrow::Cow;

/// A number literal.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub struct LitObject {
    /// An object identifier.
    pub ident: LitObjectIdent,
    /// Assignments in the object.
    pub assignments: ast::Braced<LitObjectFieldAssign, ast::Comma>,
}

impl LitObject {
    /// Parse a literal object with the given path.
    pub fn parse_with_ident(
        parser: &mut Parser<'_>,
        ident: ast::LitObjectIdent,
    ) -> Result<Self, ParseError> {
        Ok(Self {
            ident,
            assignments: parser.parse()?,
        })
    }
}

/// Parse an object literal.
///
/// # Examples
///
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::LitObject>("Foo {\"foo\": 42}");
/// testing::roundtrip::<ast::LitObject>("#{\"foo\": 42}");
/// testing::roundtrip::<ast::LitObject>("#{\"foo\": 42,}");
/// ```
impl Parse for LitObject {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let ident = parser.parse()?;
        Self::parse_with_ident(parser, ident)
    }
}

impl Peek for LitObject {
    fn peek(t1: Option<ast::Token>, t2: Option<ast::Token>) -> bool {
        match (peek!(t1).kind, peek!(t2).kind) {
            (ast::Kind::Ident(_), ast::Kind::Open(ast::Delimiter::Brace))
            | (ast::Kind::Pound, ast::Kind::Open(ast::Delimiter::Brace)) => true,
            _ => false,
        }
    }
}

/// A literal object identifier.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub enum LitObjectIdent {
    /// An anonymous object.
    Anonymous(ast::Hash),
    /// A named object.
    Named(ast::Path),
}

impl Parse for LitObjectIdent {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let token = parser.token_peek_eof()?;

        Ok(match token.kind {
            ast::Kind::Pound => Self::Anonymous(parser.parse()?),
            _ => Self::Named(parser.parse()?),
        })
    }
}

/// A literal object field.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub struct LitObjectFieldAssign {
    /// The key of the field.
    pub key: LitObjectKey,
    /// The assigned expression of the field.
    #[rune(iter)]
    pub assign: Option<(ast::Colon, ast::Expr)>,
}

/// Parse an object literal.
///
/// # Examples
///
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::LitObjectFieldAssign>("\"foo\": 42");
/// testing::roundtrip::<ast::LitObjectFieldAssign>("\"foo\": 42");
/// testing::roundtrip::<ast::LitObjectFieldAssign>("\"foo\": 42");
/// ```
impl Parse for LitObjectFieldAssign {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let key = parser.parse()?;

        let assign = if parser.peek::<ast::Colon>()? {
            let colon = parser.parse()?;
            let expr = parser.parse::<ast::Expr>()?;
            Some((colon, expr))
        } else {
            None
        };

        Ok(Self { key, assign })
    }
}

/// Possible literal object keys.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub enum LitObjectKey {
    /// A literal string (with escapes).
    LitStr(ast::LitStr),
    /// An path, usually an identifier.
    Path(ast::Path),
}

/// Parse an object literal.
///
/// # Examples
///
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::LitObjectKey>("foo");
/// testing::roundtrip::<ast::LitObjectKey>("\"foo \\n bar\"");
/// ```
impl Parse for LitObjectKey {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let token = parser.token_peek_eof()?;

        Ok(match token.kind {
            ast::Kind::LitStr { .. } => Self::LitStr(parser.parse()?),
            ast::Kind::Ident(..) => Self::Path(parser.parse()?),
            _ => {
                return Err(ParseError::expected(token, "literal object key"));
            }
        })
    }
}

/// A tag object to help peeking for anonymous object case to help
/// differentiate anonymous objects and attributes when parsing block
/// expressions.
pub struct AnonymousLitObject;

impl Peek for AnonymousLitObject {
    fn peek(t1: Option<ast::Token>, t2: Option<ast::Token>) -> bool {
        matches!(
            (peek!(t1).kind, peek!(t2).kind),
            (ast::Kind::Pound, ast::Kind::Open(ast::Delimiter::Brace))
        )
    }
}

impl<'a> Resolve<'a> for LitObjectKey {
    type Output = Cow<'a, str>;

    fn resolve(&self, storage: &Storage, source: &'a Source) -> Result<Self::Output, ParseError> {
        Ok(match self {
            Self::LitStr(lit_str) => lit_str.resolve(storage, source)?,
            Self::Path(path) => {
                let ident = path
                    .try_as_ident()
                    .ok_or_else(|| ParseError::new(path, ParseErrorKind::ExpectedObjectIdent))?;

                ident.resolve(storage, source)?
            }
        })
    }
}

impl ResolveOwned for LitObjectKey {
    type Owned = String;

    fn resolve_owned(&self, storage: &Storage, source: &Source) -> Result<Self::Owned, ParseError> {
        Ok(self.resolve(storage, source)?.into_owned())
    }
}
