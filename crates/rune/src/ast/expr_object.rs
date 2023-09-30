use crate::alloc::borrow::Cow;
use crate::ast::prelude::*;

#[test]
fn ast_parse() {
    use crate::testing::rt;

    rt::<ast::ExprObject>("Foo {\"foo\": 42}");
    rt::<ast::ExprObject>("#{\"foo\": 42}");
    rt::<ast::ExprObject>("#{\"foo\": 42,}");

    rt::<ast::FieldAssign>("\"foo\": 42");
    rt::<ast::FieldAssign>("\"foo\": 42");
    rt::<ast::FieldAssign>("\"foo\": 42");

    rt::<ast::ObjectKey>("foo");
    rt::<ast::ObjectKey>("\"foo \\n bar\"");
}

/// An object expression.
///
/// * `#{ [field]* }`.
/// * `Object { [field]* }`.
#[derive(Debug, TryClone, PartialEq, Eq, Parse, ToTokens, Spanned)]
#[non_exhaustive]
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
        match (p.nth(0), p.nth(1)) {
            (K![ident], K!['{']) => true,
            (K![#], K!['{']) => true,
            _ => false,
        }
    }
}

/// A literal object identifier.
#[derive(Debug, TryClone, PartialEq, Eq, ToTokens, Spanned)]
#[non_exhaustive]
pub enum ObjectIdent {
    /// An anonymous object.
    Anonymous(T![#]),
    /// A named object.
    Named(ast::Path),
}

impl Parse for ObjectIdent {
    fn parse(p: &mut Parser) -> Result<Self> {
        Ok(match p.nth(0)? {
            K![#] => Self::Anonymous(p.parse()?),
            _ => Self::Named(p.parse()?),
        })
    }
}

/// A single field assignment in an object expression.
#[derive(Debug, TryClone, PartialEq, Eq, ToTokens, Spanned)]
#[non_exhaustive]
pub struct FieldAssign {
    /// The key of the field.
    pub key: ObjectKey,
    /// The assigned expression of the field.
    #[rune(iter)]
    pub assign: Option<(T![:], ast::Expr)>,
}

impl Parse for FieldAssign {
    fn parse(p: &mut Parser) -> Result<Self> {
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
#[derive(Debug, TryClone, PartialEq, Eq, ToTokens, Spanned)]
#[non_exhaustive]
pub enum ObjectKey {
    /// A literal string (with escapes).
    LitStr(ast::LitStr),
    /// A path, usually an identifier.
    Path(ast::Path),
}

impl Parse for ObjectKey {
    fn parse(p: &mut Parser) -> Result<Self> {
        Ok(match p.nth(0)? {
            K![str] => Self::LitStr(p.parse()?),
            K![ident] => Self::Path(p.parse()?),
            _ => {
                return Err(compile::Error::expected(p.tok_at(0)?, "literal object key"));
            }
        })
    }
}

/// A tag object to help peeking for anonymous object case to help
/// differentiate anonymous objects and attributes when parsing block
/// expressions.
#[non_exhaustive]
pub(crate) struct AnonExprObject;

impl Peek for AnonExprObject {
    fn peek(p: &mut Peeker<'_>) -> bool {
        matches!((p.nth(0), p.nth(1)), (K![#], K!['{']))
    }
}

impl<'a> Resolve<'a> for ObjectKey {
    type Output = Cow<'a, str>;

    fn resolve(&self, cx: ResolveContext<'a>) -> Result<Self::Output> {
        Ok(match self {
            Self::LitStr(lit_str) => lit_str.resolve(cx)?,
            Self::Path(path) => {
                let ident = match path.try_as_ident() {
                    Some(ident) => ident,
                    None => {
                        return Err(compile::Error::expected(path, "object key"));
                    }
                };

                Cow::Borrowed(ident.resolve(cx)?)
            }
        })
    }
}
