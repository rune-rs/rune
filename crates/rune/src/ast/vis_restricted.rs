use crate::ast;
use crate::{Parse, Spanned, ToTokens};

/// A visibility level restricted to some path: pub(self)
/// or pub(super) or pub(crate) or pub(in some::module).
#[derive(Debug, Clone, ToTokens, Spanned, Parse)]
pub struct VisRestricted {
    /// The `pub` keyword.
    pub pub_: ast::Pub,
    /// `(` to specify the start of the visibility scope.
    pub open: ast::OpenParen,
    /// Optional `in` keyword when specifying a path scope.
    #[rune(iter)]
    pub in_: Option<ast::In>,
    /// The path in which the `pub` applies.
    pub path: ast::Path,
    /// `)` to specify the end of the path.
    pub close: ast::CloseParen,
}
