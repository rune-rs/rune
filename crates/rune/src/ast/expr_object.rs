use crate::ast;
use crate::{
    Parse, ParseError, Parser, Peek, Peeker, Resolve, ResolveError, ResolveOwned, Spanned, Storage,
    ToTokens,
};
use runestick::Source;
use std::borrow::Cow;

/// Parse an object expression.
///
/// # Examples
///
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::ExprObject>("Foo {\"foo\": 42}");
/// testing::roundtrip::<ast::ExprObject>("#{\"foo\": 42}");
/// testing::roundtrip::<ast::ExprObject>("#{\"foo\": 42,}");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Parse, ToTokens, Spanned)]
pub struct ExprObject {
    /// Attributes associated with object.
    #[rune(iter, meta)]
    pub attributes: Vec<ast::Attribute>,
    /// An object identifier.
    #[rune(meta)]
    pub ident: ObjectIdent,
    /// Assignments in the object.
    pub assignments: ast::Braced<FieldAssign, T![,]>,
}

impl Peek for ExprObject {
    fn peek(p: &mut Peeker<'_>) -> bool {
        matches!(
            (p.nth(0), p.nth(1)),
            (K![ident], K!['{']) | (K![#], K!['{'])
        )
    }
}

/// A literal object identifier.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub enum ObjectIdent {
    /// An anonymous object.
    Anonymous(T![#]),
    /// A named object.
    Named(ast::Path),
}

impl Parse for ObjectIdent {
    fn parse(p: &mut Parser) -> Result<Self, ParseError> {
        Ok(match p.nth(0)? {
            K![#] => Self::Anonymous(p.parse()?),
            _ => Self::Named(p.parse()?),
        })
    }
}

/// A literal object field.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub struct FieldAssign {
    /// The key of the field.
    pub key: ObjectKey,
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
/// testing::roundtrip::<ast::FieldAssign>("\"foo\": 42");
/// testing::roundtrip::<ast::FieldAssign>("\"foo\": 42");
/// testing::roundtrip::<ast::FieldAssign>("\"foo\": 42");
/// ```
impl Parse for FieldAssign {
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
pub enum ObjectKey {
    /// A literal string (with escapes).
    LitStr(ast::LitStr),
    /// A path, usually an identifier.
    Path(ast::Path),
}

/// Parse an object literal.
///
/// # Examples
///
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::ObjectKey>("foo");
/// testing::roundtrip::<ast::ObjectKey>("\"foo \\n bar\"");
/// ```
impl Parse for ObjectKey {
    fn parse(p: &mut Parser) -> Result<Self, ParseError> {
        Ok(match p.nth(0)? {
            K![str] => Self::LitStr(p.parse()?),
            K![ident] => Self::Path(p.parse()?),
            _ => {
                return Err(ParseError::expected(&p.tok_at(0)?, "literal object key"));
            }
        })
    }
}

/// A tag object to help peeking for anonymous object case to help
/// differentiate anonymous objects and attributes when parsing block
/// expressions.
pub struct AnonExprObject;

impl Peek for AnonExprObject {
    fn peek(p: &mut Peeker<'_>) -> bool {
        matches!((p.nth(0), p.nth(1)), (K![#], K!['{']))
    }
}

impl<'a> Resolve<'a> for ObjectKey {
    type Output = Cow<'a, str>;

    fn resolve(&self, storage: &Storage, source: &'a Source) -> Result<Self::Output, ResolveError> {
        Ok(match self {
            Self::LitStr(lit_str) => lit_str.resolve(storage, source)?,
            Self::Path(path) => {
                let ident = match path.try_as_ident() {
                    Some(ident) => ident,
                    None => {
                        return Err(ResolveError::expected(path, "object key"));
                    }
                };

                ident.resolve(storage, source)?
            }
        })
    }
}

impl ResolveOwned for ObjectKey {
    type Owned = String;

    fn resolve_owned(
        &self,
        storage: &Storage,
        source: &Source,
    ) -> Result<Self::Owned, ResolveError> {
        Ok(self.resolve(storage, source)?.into_owned())
    }
}
