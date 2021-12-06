use crate::ast::prelude::*;

/// A path, where each element is separated by a `::`.
///
/// # Examples
///
/// ```
/// use rune::{ast, testing};
///
/// testing::roundtrip::<ast::Path>("foo::bar");
/// testing::roundtrip::<ast::Path>("Self::bar");
/// testing::roundtrip::<ast::Path>("self::bar");
/// testing::roundtrip::<ast::Path>("crate::bar");
/// testing::roundtrip::<ast::Path>("super::bar");
/// testing::roundtrip::<ast::Path>("HashMap::<Foo, Bar>");
/// testing::roundtrip::<ast::Path>("super::HashMap::<Foo, Bar>");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Parse, ToTokens, Spanned, Opaque)]
#[non_exhaustive]
pub struct Path {
    /// Opaque id associated with path.
    #[rune(id)]
    pub(crate) id: Id,
    /// The optional leading colon `::` indicating global scope.
    #[rune(iter)]
    pub global: Option<T![::]>,
    /// The first component in the path.
    pub first: PathSegment,
    /// The rest of the components in the path.
    #[rune(iter)]
    pub rest: Vec<(T![::], PathSegment)>,
    /// Trailing scope.
    #[rune(iter)]
    pub trailing: Option<T![::]>,
}

impl Path {
    /// Identify the kind of the path.
    pub(crate) fn as_kind(&self) -> Option<PathKind> {
        if self.rest.is_empty() && self.trailing.is_none() && self.global.is_none() {
            match self.first {
                PathSegment::SelfValue(..) => Some(PathKind::SelfValue),
                PathSegment::Ident(ident) => Some(PathKind::Ident(ident)),
                _ => None,
            }
        } else {
            None
        }
    }

    /// Borrow as an identifier used for field access calls.
    ///
    /// This is only allowed if there are no other path components
    /// and the path segment is not `Crate` or `Super`.
    pub(crate) fn try_as_ident(&self) -> Option<&ast::Ident> {
        if self.rest.is_empty() && self.trailing.is_none() && self.global.is_none() {
            self.first.try_as_ident()
        } else {
            None
        }
    }

    /// Borrow ident and generics at the same time.
    pub(crate) fn try_as_ident_generics(
        &self,
    ) -> Option<(
        &ast::Ident,
        Option<&ast::AngleBracketed<PathSegmentExpr, T![,]>>,
    )> {
        if self.trailing.is_none() && self.global.is_none() {
            if let Some(ident) = self.first.try_as_ident() {
                let generics = if let [(_, PathSegment::Generics(generics))] = &self.rest[..] {
                    Some(generics)
                } else {
                    None
                };

                return Some((ident, generics));
            }
        }

        None
    }

    /// Borrow as an identifier used for field access calls.
    ///
    /// This is only allowed if there are no other path components
    /// and the path segment is not `Crate` or `Super`.
    pub(crate) fn try_as_ident_mut(&mut self) -> Option<&mut ast::Ident> {
        if self.rest.is_empty() && self.trailing.is_none() && self.global.is_none() {
            self.first.try_as_ident_mut()
        } else {
            None
        }
    }

    /// Iterate over all components in path.
    pub(crate) fn as_components(&self) -> impl Iterator<Item = &'_ PathSegment> + '_ {
        let mut first = Some(&self.first);
        let mut it = self.rest.iter();

        std::iter::from_fn(move || {
            if let Some(first) = first.take() {
                return Some(first);
            }

            Some(&it.next()?.1)
        })
    }
}

impl Peek for Path {
    fn peek(p: &mut Peeker<'_>) -> bool {
        matches!(p.nth(0), K![::]) || PathSegment::peek(p)
    }
}

impl IntoExpectation for &Path {
    fn into_expectation(self) -> Expectation {
        Expectation::Description("path")
    }
}

/// Resolve implementation for path which "stringifies" it.
impl<'a> Resolve<'a> for Path {
    type Output = Box<str>;

    fn resolve(&self, ctx: ResolveContext<'_>) -> Result<Self::Output, ResolveError> {
        let mut buf = String::new();

        if self.global.is_some() {
            buf.push_str("::");
        }

        match &self.first {
            PathSegment::SelfType(_) => {
                buf.push_str("Self");
            }
            PathSegment::SelfValue(_) => {
                buf.push_str("self");
            }
            PathSegment::Ident(ident) => {
                buf.push_str(ident.resolve(ctx)?);
            }
            PathSegment::Crate(_) => {
                buf.push_str("crate");
            }
            PathSegment::Super(_) => {
                buf.push_str("super");
            }
            PathSegment::Generics(_) => {
                buf.push_str("<*>");
            }
        }

        for (_, segment) in &self.rest {
            buf.push_str("::");

            match segment {
                PathSegment::SelfType(_) => {
                    buf.push_str("Self");
                }
                PathSegment::SelfValue(_) => {
                    buf.push_str("self");
                }
                PathSegment::Ident(ident) => {
                    buf.push_str(ident.resolve(ctx)?);
                }
                PathSegment::Crate(_) => {
                    buf.push_str("crate");
                }
                PathSegment::Super(_) => {
                    buf.push_str("super");
                }
                PathSegment::Generics(_) => {
                    buf.push_str("<*>");
                }
            }
        }

        if self.trailing.is_some() {
            buf.push_str("::");
        }

        Ok(buf.into_boxed_str())
    }
}

/// An identified path kind.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PathKind {
    /// A path that is the `self` value.
    SelfValue,
    /// A path that is the identifier.
    Ident(ast::Ident),
}

/// Part of a `::` separated path.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
#[non_exhaustive]
pub enum PathSegment {
    /// A path segment that contains `Self`.
    SelfType(T![Self]),
    /// A path segment that contains `self`.
    SelfValue(T![self]),
    /// A path segment that is an identifier.
    Ident(ast::Ident),
    /// The `crate` keyword used as a path segment.
    Crate(T![crate]),
    /// The `super` keyword use as a path segment.
    Super(T![super]),
    /// A path segment that is a generic argument.
    Generics(ast::AngleBracketed<PathSegmentExpr, T![,]>),
}

impl PathSegment {
    /// Borrow as an identifier.
    ///
    /// This is only allowed if the PathSegment is `Ident(_)`
    /// and not `Crate` or `Super`.
    pub(crate) fn try_as_ident(&self) -> Option<&ast::Ident> {
        if let PathSegment::Ident(ident) = self {
            Some(ident)
        } else {
            None
        }
    }

    /// Borrow as a mutable identifier.
    ///
    /// This is only allowed if the PathSegment is `Ident(_)`
    /// and not `Crate` or `Super`.
    pub(crate) fn try_as_ident_mut(&mut self) -> Option<&mut ast::Ident> {
        if let PathSegment::Ident(ident) = self {
            Some(ident)
        } else {
            None
        }
    }
}

impl IntoExpectation for PathSegment {
    fn into_expectation(self) -> Expectation {
        Expectation::Description("path segment")
    }
}

impl Parse for PathSegment {
    fn parse(p: &mut Parser<'_>) -> Result<Self, ParseError> {
        let segment = match p.nth(0)? {
            K![Self] => Self::SelfType(p.parse()?),
            K![self] => Self::SelfValue(p.parse()?),
            K![ident] => Self::Ident(p.parse()?),
            K![crate] => Self::Crate(p.parse()?),
            K![super] => Self::Super(p.parse()?),
            K![<] => Self::Generics(p.parse()?),
            _ => {
                return Err(ParseError::expected(p.tok_at(0)?, "path segment"));
            }
        };

        Ok(segment)
    }
}

impl Peek for PathSegment {
    fn peek(p: &mut Peeker<'_>) -> bool {
        matches!(
            p.nth(0),
            K![<] | K![Self] | K![self] | K![crate] | K![super] | K![ident]
        )
    }
}

/// Used to parse an expression without supporting an immediate binary expression.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
#[non_exhaustive]
pub struct PathSegmentExpr {
    /// The expression that makes up the path segment.
    pub expr: ast::Expr,
}

impl Parse for PathSegmentExpr {
    fn parse(p: &mut Parser) -> Result<Self, ParseError> {
        let expr = ast::Expr::parse_with(
            p,
            ast::expr::NOT_EAGER_BRACE,
            ast::expr::NOT_EAGER_BINARY,
            ast::expr::NOT_CALLABLE,
        )?;

        Ok(Self { expr })
    }
}

impl Peek for PathSegmentExpr {
    fn peek(p: &mut Peeker<'_>) -> bool {
        ast::Expr::peek(p)
    }
}
