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

impl Pat {
    /// Parse a pattern with a starting identifier.
    pub fn parse_ident(
        parser: &mut Parser,
        attributes: Vec<ast::Attribute>,
    ) -> Result<Self, ParseError> {
        let path: ast::Path = parser.parse()?;

        Ok(match parser.nth(0)? {
            K!['('] => Self::PatTuple(PatTuple {
                attributes,
                path: Some(path),
                items: parser.parse()?,
            }),
            K!['{'] => {
                let ident = ast::LitObjectIdent::Named(path);

                Self::PatObject(PatObject {
                    attributes,
                    ident,
                    items: parser.parse()?,
                })
            }
            K![:] => Self::PatBinding(PatBinding {
                attributes,
                key: ast::LitObjectKey::Path(path),
                colon: parser.parse()?,
                pat: parser.parse()?,
            }),
            _ => Self::PatPath(PatPath { attributes, path }),
        })
    }
}

/// Parsing a block expression.
///
/// # Examples
///
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::Pat>("()");
/// testing::roundtrip::<ast::Pat>("1");
/// testing::roundtrip::<ast::Pat>("'a'");
/// testing::roundtrip::<ast::Pat>("\"hello world\"");
/// testing::roundtrip::<ast::Pat>("var");
/// testing::roundtrip::<ast::Pat>("_");
/// testing::roundtrip::<ast::Pat>("Foo(n)");
/// ```
impl Parse for Pat {
    fn parse(p: &mut Parser<'_>) -> Result<Self, ParseError> {
        let attributes = p.parse::<Vec<ast::Attribute>>()?;

        match p.nth(0)? {
            ast::Kind::LitStr(..) => {
                let lit_str = p.parse::<ast::LitStr>()?;

                return Ok(if p.peek::<T![:]>()? {
                    Self::PatBinding(PatBinding {
                        attributes,
                        key: ast::LitObjectKey::LitStr(lit_str),
                        colon: p.parse()?,
                        pat: p.parse()?,
                    })
                } else {
                    Self::PatLit(PatLit {
                        attributes,
                        expr: Box::new(ast::Expr::ExprLit(ast::ExprLit {
                            attributes: vec![],
                            lit: ast::Lit::Str(lit_str),
                        })),
                    })
                });
            }
            K![..] => {
                return Ok(Self::PatRest(PatRest {
                    attributes,
                    dot_dot: p.parse()?,
                }))
            }
            K!['('] => {
                return Ok(if p.peek::<ast::LitUnit>()? {
                    Self::PatLit(PatLit {
                        attributes,
                        expr: Box::new(ast::Expr::ExprLit(ast::ExprLit {
                            attributes: vec![],
                            lit: ast::Lit::Unit(p.parse()?),
                        })),
                    })
                } else {
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
            ast::Kind::LitByte { .. }
            | ast::Kind::LitChar { .. }
            | ast::Kind::LitNumber { .. }
            | K![-] => {
                let expr: ast::Expr = p.parse()?;

                match &expr {
                    ast::Expr::ExprLit(..) => {
                        return Ok(Self::PatLit(PatLit {
                            attributes,
                            expr: Box::new(expr),
                        }));
                    }
                    ast::Expr::ExprUnary(ast::ExprUnary {
                        op: ast::UnOp::Neg,
                        expr: unary_expr,
                        ..
                    }) => {
                        if let ast::Expr::ExprLit(ast::ExprLit {
                            lit: ast::Lit::Number(..),
                            ..
                        }) = &**unary_expr
                        {
                            return Ok(Self::PatLit(PatLit {
                                attributes,
                                expr: Box::new(expr),
                            }));
                        }
                    }
                    _ => (),
                }
            }
            K![_] => {
                return Ok(Self::PatIgnore(PatIgnore {
                    attributes,
                    underscore: p.parse()?,
                }))
            }
            K![ident(..)] => return Ok(Self::parse_ident(p, attributes)?),
            _ => (),
        }

        Err(ParseError::expected(p.token(0)?, "pattern"))
    }
}

impl Peek for Pat {
    fn peek(p: &mut Peeker<'_>) -> bool {
        match p.nth(0) {
            K!['('] => true,
            K!['['] => true,
            K![#] => true,
            K![_] => true,
            ast::Kind::LitByte { .. } => true,
            ast::Kind::LitChar { .. } => true,
            ast::Kind::LitNumber { .. } => true,
            ast::Kind::LitStr { .. } => true,
            ast::Kind::Ident(..) => true,
            _ => false,
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
    pub expr: Box<ast::Expr>,
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
    pub ident: ast::LitObjectIdent,
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
    pub key: ast::LitObjectKey,
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
