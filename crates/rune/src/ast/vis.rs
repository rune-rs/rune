use crate::ast;
use crate::{Parse, ParseError, ParseErrorKind, Parser, Peek, Spanned, ToTokens};

/// Visibility level restricted to some path: pub(self) or pub(super) or pub(crate) or pub(in some::module).
#[derive(Debug, Clone, ToTokens, Spanned)]
pub enum Visibility {
    /// A public visibility level: `pub`.
    Public(ast::Pub),
    /// A public visibility level: `priv`.
    /// Private(ast::Priv),
    /// A public visibility level: `crate`.
    Crate(ast::Crate),
    /// An inherited visibility, which usually means private.
    Inherited(ast::Priv),
    /// A visibility level restricted to some path: `pub(self)` or
    /// `pub(super)` or `pub(crate)` or `pub(in some::module)`.
    Restricted(ast::VisRestricted),
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
/// match parse_all::<ast::Visibility>("crate").unwrap() {
///     ast::Visibility::Crate(_) => {}
///     vis => panic!("expected `Crate` visibility got {:?}", vis),
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
/// match parse_all::<ast::Visibility>("priv").unwrap() {
///     ast::Visibility::Inherited(_) => {}
///     vis => panic!("expected `Inherited` visibility got {:?}", vis),
/// }
///
/// ```
impl Parse for Visibility {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let token = parser.token_peek_eof()?;

        match token.kind {
            ast::Kind::Pub => {
                let pub_ = parser.parse()?;
                let next_kind = parser.token_peek()?.map(|t| t.kind);

                if let Some(ast::Kind::Open(ast::Delimiter::Parenthesis)) = next_kind {
                    Ok(Visibility::Restricted(ast::VisRestricted {
                        pub_,
                        open: parser.parse()?,
                        in_: parser.parse()?,
                        path: parser.parse()?,
                        close: parser.parse()?,
                    }))
                } else {
                    Ok(Visibility::Public(pub_))
                }
            }
            ast::Kind::Priv => Ok(ast::Visibility::Inherited(parser.parse()?)),
            ast::Kind::Crate => Ok(ast::Visibility::Crate(parser.parse()?)),
            _ => Err(ParseError::new(
                token,
                ParseErrorKind::ExpectedVisibility { actual: token.kind },
            )),
        }
    }
}

impl Peek for Visibility {
    fn peek(t1: Option<ast::Token>, _t2: Option<ast::Token>) -> bool {
        matches!(peek!(t1).kind, ast::Kind::Pub | ast::Kind::Crate)
    }
}
