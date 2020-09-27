use crate::ast;
use crate::{Parse, Spanned, ToTokens};

/// A return statement `<expr>.await`.
///
/// # Examples
///
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::Expr>("(42).await");
/// testing::roundtrip::<ast::Expr>("self.await");
/// testing::roundtrip::<ast::Expr>("test.await");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Parse, Spanned)]
pub struct ExprAwait {
    /// Attributes associated with expression.
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The expression being awaited.
    pub expr: Box<ast::Expr>,
    /// The dot separating the expression.
    pub dot: ast::Dot,
    /// The await token.
    pub await_: ast::Await,
}
