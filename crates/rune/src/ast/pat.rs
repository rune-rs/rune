use crate::ast;
use crate::{Parse, ParseError, Parser, Peek, Spanned, ToTokens};

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

        let t = match parser.token_peek()? {
            Some(t) => t,
            None => return Ok(Self::PatPath(ast::PatPath { attributes, path })),
        };

        Ok(match t.kind {
            ast::Kind::Open(ast::Delimiter::Parenthesis) => Self::PatTuple(PatTuple {
                attributes,
                path: Some(path),
                items: parser.parse()?,
            }),
            ast::Kind::Open(ast::Delimiter::Brace) => {
                let ident = ast::LitObjectIdent::Named(path);

                Self::PatObject(PatObject {
                    attributes,
                    ident,
                    items: parser.parse()?,
                })
            }
            ast::Kind::Colon => Self::PatBinding(PatBinding {
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
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let token = parser.token_peek_eof()?;

        let attributes = parser.parse::<Vec<ast::Attribute>>()?;

        match token.kind {
            ast::Kind::LitStr(..) => {
                let lit_str = parser.parse::<ast::LitStr>()?;

                return Ok(if parser.peek::<ast::Colon>()? {
                    Self::PatBinding(PatBinding {
                        attributes,
                        key: ast::LitObjectKey::LitStr(lit_str),
                        colon: parser.parse()?,
                        pat: parser.parse()?,
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
            ast::Kind::DotDot => {
                return Ok(Self::PatRest(PatRest {
                    attributes,
                    dot_dot: parser.parse()?,
                }))
            }
            ast::Kind::Open(ast::Delimiter::Parenthesis) => {
                return Ok(if parser.peek::<ast::LitUnit>()? {
                    Self::PatLit(PatLit {
                        attributes,
                        expr: Box::new(ast::Expr::ExprLit(ast::ExprLit {
                            attributes: vec![],
                            lit: ast::Lit::Unit(parser.parse()?),
                        })),
                    })
                } else {
                    Self::PatTuple(PatTuple {
                        attributes,
                        path: None,
                        items: parser.parse()?,
                    })
                });
            }
            ast::Kind::Open(ast::Delimiter::Bracket) => {
                return Ok(Self::PatVec(PatVec {
                    attributes,
                    items: parser.parse()?,
                }))
            }
            ast::Kind::Pound => {
                return Ok(Self::PatObject(PatObject {
                    attributes,
                    ident: parser.parse()?,
                    items: parser.parse()?,
                }))
            }
            ast::Kind::LitByte { .. }
            | ast::Kind::LitChar { .. }
            | ast::Kind::LitNumber { .. }
            | ast::Kind::Dash => {
                let expr: ast::Expr = parser.parse()?;

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
            ast::Kind::Underscore => {
                return Ok(Self::PatIgnore(PatIgnore {
                    attributes,
                    underscore: parser.parse()?,
                }))
            }
            ast::Kind::Ident(..) => return Ok(Self::parse_ident(parser, attributes)?),
            _ => (),
        }

        Err(ParseError::expected(token, "pattern"))
    }
}

impl Peek for Pat {
    fn peek(t1: Option<ast::Token>, _: Option<ast::Token>) -> bool {
        match peek!(t1).kind {
            ast::Kind::Open(ast::Delimiter::Parenthesis) => true,
            ast::Kind::Open(ast::Delimiter::Bracket) => true,
            ast::Kind::Pound => true,
            ast::Kind::LitByte { .. } => true,
            ast::Kind::LitChar { .. } => true,
            ast::Kind::LitNumber { .. } => true,
            ast::Kind::LitStr { .. } => true,
            ast::Kind::Underscore => true,
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
    pub dot_dot: ast::DotDot,
}

/// An array pattern.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub struct PatVec {
    /// Attributes associated with the vector pattern.
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// Bracketed patterns.
    pub items: ast::Bracketed<ast::Pat, ast::Comma>,
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
    pub items: ast::Parenthesized<ast::Pat, ast::Comma>,
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
    pub items: ast::Braced<Pat, ast::Comma>,
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
    pub colon: ast::Colon,
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
    pub underscore: ast::Underscore,
}
