use crate::ast::prelude::*;

#[test]
fn ast_parse() {
    use crate::testing::rt;

    assert!(matches! {
        rt::<ast::Visibility>("pub"),
        ast::Visibility::Public(_)
    });

    assert!(matches! {
        rt::<ast::Visibility>("pub (in a::b::c)"),
        ast::Visibility::In(_)
    });

    assert!(matches! {
        rt::<ast::Visibility>("pub(in crate::x::y::z)"),
        ast::Visibility::In(_)
    });

    assert!(matches! {
        rt::<ast::Visibility>("pub(super)"),
        ast::Visibility::Super(_)
    });

    assert!(matches! {
        rt::<ast::Visibility>("pub(crate)"),
        ast::Visibility::Crate(_)
    });

    assert!(matches! {
        rt::<ast::Visibility>("pub(self)"),
        ast::Visibility::SelfValue(_)
    });
}

/// Visibility level restricted to some path.
///
/// * `pub(self)`.
/// * `pub(super)`.
/// * `pub(crate)`.
/// * `pub(in some::module)`.
#[derive(Debug, TryClone, PartialEq, Eq, ToTokens, OptionSpanned)]
#[non_exhaustive]
pub enum Visibility {
    /// An inherited visibility level, this usually means private.
    Inherited,
    /// An unrestricted public visibility level: `pub`.
    Public(T![pub]),
    /// Crate visibility `pub(crate)`.
    Crate(VisibilityRestrict<T![crate]>),
    /// Super visibility `pub(super)`.
    Super(VisibilityRestrict<T![super]>),
    /// Self visibility `pub(self)`.
    SelfValue(VisibilityRestrict<T![self]>),
    /// In visibility `pub(in path)`.
    In(VisibilityRestrict<VisibilityIn>),
}

impl Visibility {
    /// Return `true` if it is the `Inherited` variant
    pub const fn is_inherited(&self) -> bool {
        matches!(self, Visibility::Inherited)
    }

    /// Return `true` if the module is public.
    pub const fn is_public(&self) -> bool {
        matches!(self, Visibility::Public(..))
    }
}

impl Default for Visibility {
    fn default() -> Self {
        Self::Inherited
    }
}

impl Parse for Visibility {
    fn parse(parser: &mut Parser<'_>) -> Result<Self> {
        let pub_token = match parser.parse::<Option<T![pub]>>()? {
            Some(pub_token) => pub_token,
            None => return Ok(Self::Inherited),
        };

        let open = match parser.parse::<Option<ast::OpenParen>>()? {
            Some(open) => open,
            None => return Ok(Self::Public(pub_token)),
        };

        Ok(match parser.nth(0)? {
            K![in] => Self::In(VisibilityRestrict {
                pub_token,
                open,
                restriction: VisibilityIn {
                    in_token: parser.parse()?,
                    path: parser.parse()?,
                },
                close: parser.parse()?,
            }),
            K![super] => Self::Super(VisibilityRestrict {
                pub_token,
                open,
                restriction: parser.parse()?,
                close: parser.parse()?,
            }),
            K![self] => Self::SelfValue(VisibilityRestrict {
                pub_token,
                open,
                restriction: parser.parse()?,
                close: parser.parse()?,
            }),
            _ => Self::Crate(VisibilityRestrict {
                pub_token,
                open,
                restriction: parser.parse()?,
                close: parser.parse()?,
            }),
        })
    }
}

/// A `in path` restriction to visibility.
#[derive(Debug, TryClone, PartialEq, Eq, ToTokens, Spanned)]
#[non_exhaustive]
pub struct VisibilityIn {
    /// The `in` keyword.
    pub in_token: T![in],
    /// The path the restriction applies to.
    pub path: ast::Path,
}

/// A restriction to visibility.
#[derive(Debug, TryClone, PartialEq, Eq, ToTokens, Spanned)]
#[non_exhaustive]
#[try_clone(bound = {T: TryClone})]
pub struct VisibilityRestrict<T> {
    /// `pub` keyword.
    pub pub_token: ast::generated::Pub,
    /// Opening paren `(`.
    pub open: ast::OpenParen,
    /// The restriction.
    pub restriction: T,
    /// Closing paren `)`.
    pub close: ast::CloseParen,
}
