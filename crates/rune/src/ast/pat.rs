use crate::ast;
use crate::{Parse, ParseError, Parser, Peek, Peeker, Spanned, ToTokens};

/// A pattern match.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub enum Pat {
    /// An ignored binding `_`.
    PatIgnore(PatIgnore),
    /// A variable binding `n`.
    PatPath(PatPath),
    /// A literal pattern. This is represented as an expression.
    PatLit(PatLit),
    /// A vector pattern.
    PatVec(PatVec),
    /// A tuple pattern.
    PatTuple(PatTuple),
    /// An object pattern.
    PatObject(PatObject),
    /// A binding `a: pattern` or `"foo": pattern`.
    PatBinding(PatBinding),
    /// The rest pattern `..`.
    PatRest(PatRest),
}

/// Parsing a block expression.
///
/// # Examples
///
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::Pat>("()");
/// testing::roundtrip::<ast::Pat>("42");
/// testing::roundtrip::<ast::Pat>("-42");
/// testing::roundtrip::<ast::Pat>("3.1415");
/// testing::roundtrip::<ast::Pat>("-3.1415");
/// testing::roundtrip::<ast::Pat>("b'a'");
/// testing::roundtrip::<ast::Pat>("'a'");
/// testing::roundtrip::<ast::Pat>("b\"hello world\"");
/// testing::roundtrip::<ast::Pat>("\"hello world\"");
/// testing::roundtrip::<ast::Pat>("var");
/// testing::roundtrip::<ast::Pat>("_");
/// testing::roundtrip::<ast::Pat>("Foo(n)");
/// ```
impl Parse for Pat {
    fn parse(p: &mut Parser<'_>) -> Result<Self, ParseError> {
        let attributes = p.parse::<Vec<ast::Attribute>>()?;

        match p.nth(0)? {
            K![byte] => {
                return Ok(Self::PatLit(PatLit {
                    attributes,
                    expr: ast::Expr::from_lit(ast::Lit::Byte(p.parse()?)),
                }));
            }
            K![char] => {
                return Ok(Self::PatLit(PatLit {
                    attributes,
                    expr: ast::Expr::from_lit(ast::Lit::Char(p.parse()?)),
                }));
            }
            K![bytestr] => {
                return Ok(Self::PatLit(PatLit {
                    attributes,
                    expr: ast::Expr::from_lit(ast::Lit::ByteStr(p.parse()?)),
                }));
            }
            K![true] | K![false] => {
                return Ok(Self::PatLit(PatLit {
                    attributes,
                    expr: ast::Expr::from_lit(ast::Lit::Bool(p.parse()?)),
                }));
            }
            K![str] => {
                return Ok(match p.nth(1)? {
                    K![:] => Self::PatBinding(PatBinding {
                        attributes,
                        key: ast::ObjectKey::LitStr(p.parse()?),
                        colon: p.parse()?,
                        pat: p.parse()?,
                    }),
                    _ => Self::PatLit(PatLit {
                        attributes,
                        expr: ast::Expr::from_lit(ast::Lit::Str(p.parse()?)),
                    }),
                });
            }
            K![number] => {
                return Ok(Self::PatLit(PatLit {
                    attributes,
                    expr: ast::Expr::from_lit(ast::Lit::Number(p.parse()?)),
                }));
            }
            K![..] => {
                return Ok(Self::PatRest(PatRest {
                    attributes,
                    dot_dot: p.parse()?,
                }))
            }
            K!['('] => {
                return Ok({
                    let _nth = p.nth(1)?;

                    Self::PatTuple(PatTuple {
                        attributes,
                        path: None,
                        items: p.parse()?,
                    })
                });
            }
            K!['['] => {
                return Ok(Self::PatVec(PatVec {
                    attributes,
                    items: p.parse()?,
                }))
            }
            K![#] => {
                return Ok(Self::PatObject(PatObject {
                    attributes,
                    ident: p.parse()?,
                    items: p.parse()?,
                }))
            }
            K![-] => {
                let expr: ast::Expr = p.parse()?;

                if expr.is_lit() {
                    return Ok(Self::PatLit(PatLit { attributes, expr }));
                }
            }
            K![_] => {
                return Ok(Self::PatIgnore(PatIgnore {
                    attributes,
                    underscore: p.parse()?,
                }))
            }
            _ if ast::Path::peek(p.peeker()) => {
                let path = p.parse::<ast::Path>()?;

                return Ok(match p.nth(0)? {
                    K!['('] => Self::PatTuple(PatTuple {
                        attributes,
                        path: Some(path),
                        items: p.parse()?,
                    }),
                    K!['{'] => Self::PatObject(PatObject {
                        attributes,
                        ident: ast::ObjectIdent::Named(path),
                        items: p.parse()?,
                    }),
                    K![:] => Self::PatBinding(PatBinding {
                        attributes,
                        key: ast::ObjectKey::Path(path),
                        colon: p.parse()?,
                        pat: p.parse()?,
                    }),
                    _ => Self::PatPath(PatPath { attributes, path }),
                });
            }
            _ => (),
        }

        Err(ParseError::expected(&p.tok_at(0)?, "pattern"))
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
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub struct PatLit {
    /// Attributes associated with the pattern.
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The literal expression.
    pub expr: ast::Expr,
}

/// The rest pattern `..` and associated attributes.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub struct PatRest {
    /// Attribute associated with the rest pattern.
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The rest token `..`.
    pub dot_dot: T![..],
}

/// An array pattern.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub struct PatVec {
    /// Attributes associated with the vector pattern.
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// Bracketed patterns.
    pub items: ast::Bracketed<ast::Pat, T![,]>,
}

/// A tuple pattern.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
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
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
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
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned, Parse)]
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

/// A tuple pattern.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub struct PatPath {
    /// Attributes associate with the path.
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The path, if the tuple is typed.
    pub path: ast::Path,
}

/// A ignore pattern.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub struct PatIgnore {
    /// Attributes associate with the path.
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The ignore token`_`.
    pub underscore: T![_],
}
