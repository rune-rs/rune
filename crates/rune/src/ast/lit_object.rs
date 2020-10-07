use crate::ast;
use crate::{
    Parse, ParseError, Parser, Peek, Peeker, Resolve, ResolveOwned, Spanned, Storage, ToTokens,
};
use runestick::Source;
use std::borrow::Cow;

/// A number literal.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub struct LitObject {
    /// An object identifier.
    pub ident: LitObjectIdent,
    /// Assignments in the object.
    pub assignments: ast::Braced<LitObjectFieldAssign, T![,]>,
}

impl LitObject {
    /// Parse a literal object with the given path.
    pub fn parse_with_ident(
        p: &mut Parser<'_>,
        ident: ast::LitObjectIdent,
    ) -> Result<Self, ParseError> {
        Ok(Self {
            ident,
            assignments: p.parse()?,
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
    fn parse(p: &mut Parser) -> Result<Self, ParseError> {
        let ident = p.parse()?;
        Self::parse_with_ident(p, ident)
    }
}

impl Peek for LitObject {
    fn peek(p: &mut Peeker<'_>) -> bool {
        match (p.nth(0), p.nth(1)) {
            (K![ident], K!['{']) => true,
            (K![#], K!['{']) => true,
            _ => false,
        }
    }
}

/// A literal object identifier.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub enum LitObjectIdent {
    /// An anonymous object.
    Anonymous(T![#]),
    /// A named object.
    Named(ast::Path),
}

impl Parse for LitObjectIdent {
    fn parse(p: &mut Parser) -> Result<Self, ParseError> {
        Ok(match p.nth(0)? {
            K![#] => Self::Anonymous(p.parse()?),
            _ => Self::Named(p.parse()?),
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
    pub assign: Option<(T![:], ast::Expr)>,
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
    fn parse(p: &mut Parser) -> Result<Self, ParseError> {
        let key = p.parse()?;

        let assign = if p.peek::<T![:]>()? {
            let colon = p.parse()?;
            let expr = p.parse::<ast::Expr>()?;
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
    fn parse(p: &mut Parser) -> Result<Self, ParseError> {
        Ok(match p.nth(0)? {
            K![str] => Self::LitStr(p.parse()?),
            K![ident] => Self::Path(p.parse()?),
            _ => {
                return Err(ParseError::expected(&p.token(0)?, "literal object key"));
            }
        })
    }
}

/// A tag object to help peeking for anonymous object case to help
/// differentiate anonymous objects and attributes when parsing block
/// expressions.
pub struct AnonymousLitObject;

impl Peek for AnonymousLitObject {
    fn peek(p: &mut Peeker<'_>) -> bool {
        matches!((p.nth(0), p.nth(1)), (K![#], K!['{']))
    }
}

impl<'a> Resolve<'a> for LitObjectKey {
    type Output = Cow<'a, str>;

    fn resolve(&self, storage: &Storage, source: &'a Source) -> Result<Self::Output, ParseError> {
        Ok(match self {
            Self::LitStr(lit_str) => lit_str.resolve(storage, source)?,
            Self::Path(path) => {
                let ident = match path.try_as_ident() {
                    Some(ident) => ident,
                    None => {
                        return Err(ParseError::expected(path, "object key"));
                    }
                };

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
