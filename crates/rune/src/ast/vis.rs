use crate::ast;
use crate::{OptionSpanned, Parse, ParseError, Parser, ToTokens};

/// Visibility level restricted to some path: pub(self) or pub(super) or pub(crate) or pub(in some::module).
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, OptionSpanned)]
pub enum Visibility {
    /// An unrestricted public visibility level: `pub`.
    Public(ast::Pub),
    /// A visibility level restricted to some path: `pub(self)` or
    /// `pub(super)` or `pub(crate)` or `pub(in some::module)`.
    Restricted(ast::VisRestricted),
    /// An inherited visibility level, this usually means private.
    Inherited,
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

/// Parsing Visibility specifiers
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast, ParseError};
///
/// match parse_all::<ast::Visibility>("pub").unwrap() {
///     ast::Visibility::Public(_) => {}
///     vis => panic!("expected `Public` visibility got {:?}", vis),
/// }
///
/// match parse_all::<ast::Visibility>("pub(in a::b::c)").unwrap() {
///     ast::Visibility::Restricted(ast::VisRestricted{..}) => {}
///     vis => panic!("expected `Restricted` visibility got {:?}", vis),
/// }
///
/// match parse_all::<ast::Visibility>("pub(in crate::x::y::z)").unwrap() {
///     ast::Visibility::Restricted(ast::VisRestricted{..}) => {}
///     vis => panic!("expected `Restricted` visibility got {:?}", vis),
/// }
///
/// match parse_all::<ast::Visibility>("pub(super)").unwrap() {
///     ast::Visibility::Restricted(ast::VisRestricted{..}) => {}
///     vis => panic!("expected `Restricted` visibility got {:?}", vis),
/// }
///
/// match parse_all::<ast::Visibility>("pub(crate)").unwrap() {
///     ast::Visibility::Restricted(ast::VisRestricted{..}) => {}
///     vis => panic!("expected `Restricted` visibility got {:?}", vis),
/// }
///
///
/// ```
impl Parse for Visibility {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let token = match parser.token_peek()? {
            Some(token) => token,
            None => return Ok(Self::Inherited),
        };

        if token.kind == ast::Kind::Pub {
            let pub_ = parser.parse()?;
            let next_kind = parser.token_peek()?.map(|t| t.kind);
            match next_kind {
                Some(ast::Kind::Open(ast::Delimiter::Parenthesis)) => {
                    Ok(Visibility::Restricted(ast::VisRestricted {
                        pub_,
                        open: parser.parse()?,
                        in_: parser.parse()?,
                        path: parser.parse()?,
                        close: parser.parse()?,
                    }))
                }
                _ => Ok(Visibility::Public(pub_)),
            }
        } else {
            Ok(Visibility::Inherited)
        }
    }
}
