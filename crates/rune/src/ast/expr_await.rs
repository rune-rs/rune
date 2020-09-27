use crate::ast;
use crate::{Parse, Spanned, ToTokens};

/// A return statement `<expr>.await`.
///
/// # Examples
///
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::ExprAwait>("42.await");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Parse, Spanned)]
pub struct ExprAwait {
    /// The expression being awaited.
    pub expr: Box<ast::Expr>,
    /// The dot separating the expression.
    pub dot: ast::Dot,
    /// The await token.
    pub await_: ast::Await,
}
