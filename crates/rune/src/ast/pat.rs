use crate::ast::prelude::*;

#[test]
fn ast_parse() {
    use crate::testing::rt;

    rt::<ast::Pat>("()");
    rt::<ast::Pat>("42");
    rt::<ast::Pat>("-42");
    rt::<ast::Pat>("3.1415");
    rt::<ast::Pat>("-3.1415");
    rt::<ast::Pat>("b'a'");
    rt::<ast::Pat>("'a'");
    rt::<ast::Pat>("b\"hello world\"");
    rt::<ast::Pat>("\"hello world\"");
    rt::<ast::Pat>("var");
    rt::<ast::Pat>("_");
    rt::<ast::Pat>("Foo(n)");
}

/// A pattern match.
#[derive(Debug, TryClone, PartialEq, Eq, ToTokens, Spanned)]
#[non_exhaustive]
pub enum Pat {
    /// An ignored binding `_`.
    Ignore(PatIgnore),
    /// A variable binding `n`.
    Path(PatPath),
    /// A literal pattern. This is represented as an expression.
    Lit(PatLit),
    /// A vector pattern.
    Vec(PatVec),
    /// A tuple pattern.
    Tuple(PatTuple),
    /// An object pattern.
    Object(PatObject),
    /// A binding `a: pattern` or `"foo": pattern`.
    Binding(PatBinding),
    /// The rest pattern `..`.
    Rest(PatRest),
}

impl Parse for Pat {
    fn parse(p: &mut Parser<'_>) -> Result<Self> {
        let attributes = p.parse::<Vec<ast::Attribute>>()?;

        match p.nth(0)? {
            K![byte] => {
                return Ok(Self::Lit(PatLit {
                    attributes,
                    expr: Box::try_new(ast::Expr::from_lit(ast::Lit::Byte(p.parse()?)))?,
                }));
            }
            K![char] => {
                return Ok(Self::Lit(PatLit {
                    attributes,
                    expr: Box::try_new(ast::Expr::from_lit(ast::Lit::Char(p.parse()?)))?,
                }));
            }
            K![bytestr] => {
                return Ok(Self::Lit(PatLit {
                    attributes,
                    expr: Box::try_new(ast::Expr::from_lit(ast::Lit::ByteStr(p.parse()?)))?,
                }));
            }
            K![true] | K![false] => {
                return Ok(Self::Lit(PatLit {
                    attributes,
                    expr: Box::try_new(ast::Expr::from_lit(ast::Lit::Bool(p.parse()?)))?,
                }));
            }
            K![str] => {
                return Ok(match p.nth(1)? {
                    K![:] => Self::Binding(PatBinding {
                        attributes,
                        key: ast::ObjectKey::LitStr(p.parse()?),
                        colon: p.parse()?,
                        pat: p.parse()?,
                    }),
                    _ => Self::Lit(PatLit {
                        attributes,
                        expr: Box::try_new(ast::Expr::from_lit(ast::Lit::Str(p.parse()?)))?,
                    }),
                });
            }
            K![number] => {
                return Ok(Self::Lit(PatLit {
                    attributes,
                    expr: Box::try_new(ast::Expr::from_lit(ast::Lit::Number(p.parse()?)))?,
                }));
            }
            K![..] => {
                return Ok(Self::Rest(PatRest {
                    attributes,
                    dot_dot: p.parse()?,
                }))
            }
            K!['('] => {
                return Ok({
                    let _nth = p.nth(1)?;

                    Self::Tuple(PatTuple {
                        attributes,
                        path: None,
                        items: p.parse()?,
                    })
                });
            }
            K!['['] => {
                return Ok(Self::Vec(PatVec {
                    attributes,
                    items: p.parse()?,
                }))
            }
            K![#] => {
                return Ok(Self::Object(PatObject {
                    attributes,
                    ident: p.parse()?,
                    items: p.parse()?,
                }))
            }
            K![-] => {
                let expr: ast::Expr = p.parse()?;

                if expr.is_lit() {
                    return Ok(Self::Lit(PatLit {
                        attributes,
                        expr: Box::try_new(expr)?,
                    }));
                }
            }
            K![_] => {
                return Ok(Self::Ignore(PatIgnore {
                    attributes,
                    underscore: p.parse()?,
                }))
            }
            _ if ast::Path::peek(p.peeker()) => {
                let path = p.parse::<ast::Path>()?;

                return Ok(match p.nth(0)? {
                    K!['('] => Self::Tuple(PatTuple {
                        attributes,
                        path: Some(path),
                        items: p.parse()?,
                    }),
                    K!['{'] => Self::Object(PatObject {
                        attributes,
                        ident: ast::ObjectIdent::Named(path),
                        items: p.parse()?,
                    }),
                    K![:] => Self::Binding(PatBinding {
                        attributes,
                        key: ast::ObjectKey::Path(path),
                        colon: p.parse()?,
                        pat: p.parse()?,
                    }),
                    _ => Self::Path(PatPath { attributes, path }),
                });
            }
            _ => (),
        }

        Err(compile::Error::expected(p.tok_at(0)?, "pattern"))
    }
}

impl Peek for Pat {
    fn peek(p: &mut Peeker<'_>) -> bool {
        match p.nth(0) {
            K!['('] => true,
            K!['['] => true,
            K![#] => matches!(p.nth(1), K!['{']),
            K![_] => true,
            K![..] => true,
            K![byte] | K![char] | K![number] | K![str] => true,
            K![true] | K![false] => true,
            K![-] => matches!(p.nth(1), K![number]),
            _ => ast::Path::peek(p),
        }
    }
}

/// A literal pattern.
#[derive(Debug, TryClone, PartialEq, Eq, ToTokens, Spanned)]
#[non_exhaustive]
pub struct PatLit {
    /// Attributes associated with the pattern.
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The literal expression.
    pub expr: Box<ast::Expr>,
}

/// The rest pattern `..` and associated attributes.
#[derive(Debug, TryClone, PartialEq, Eq, ToTokens, Spanned)]
#[non_exhaustive]
pub struct PatRest {
    /// Attribute associated with the rest pattern.
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The rest token `..`.
    pub dot_dot: T![..],
}

/// An array pattern.
#[derive(Debug, TryClone, PartialEq, Eq, ToTokens, Spanned)]
#[non_exhaustive]
pub struct PatVec {
    /// Attributes associated with the vector pattern.
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// Bracketed patterns.
    pub items: ast::Bracketed<ast::Pat, T![,]>,
}

/// A tuple pattern.
#[derive(Debug, TryClone, PartialEq, Eq, ToTokens, Spanned)]
#[non_exhaustive]
pub struct PatTuple {
    /// Attributes associated with the object pattern.
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The path, if the tuple is typed.
    #[rune(iter)]
    pub path: Option<ast::Path>,
    /// The items in the tuple.
    pub items: ast::Parenthesized<ast::Pat, T![,]>,
}

/// An object pattern.
#[derive(Debug, TryClone, PartialEq, Eq, ToTokens, Spanned)]
#[non_exhaustive]
pub struct PatObject {
    /// Attributes associated with the object pattern.
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The identifier of the object pattern.
    pub ident: ast::ObjectIdent,
    /// The fields matched against.
    pub items: ast::Braced<Pat, T![,]>,
}

/// An object item.
#[derive(Debug, TryClone, PartialEq, Eq, ToTokens, Spanned, Parse)]
#[non_exhaustive]
pub struct PatBinding {
    /// Attributes associate with the binding.
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The key of an object.
    pub key: ast::ObjectKey,
    /// The colon separator for the binding.
    pub colon: T![:],
    /// What the binding is to.
    pub pat: Box<ast::Pat>,
}

/// A path pattern.
#[derive(Debug, TryClone, PartialEq, Eq, ToTokens, Spanned)]
#[non_exhaustive]
pub struct PatPath {
    /// Attributes associate with the path.
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The path of the pattern.
    pub path: ast::Path,
}

/// An ignore pattern.
#[derive(Debug, TryClone, PartialEq, Eq, ToTokens, Spanned)]
#[non_exhaustive]
pub struct PatIgnore {
    /// Attributes associate with the pattern.
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The ignore token`_`.
    pub underscore: T![_],
}
