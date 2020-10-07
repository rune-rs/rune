use crate::ast;
use crate::{Spanned, ToTokens};

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
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub struct ExprAwait {
    /// Attributes associated with expression.
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The expression being awaited.
    pub expr: ast::Expr,
    /// The dot separating the expression.
    pub dot: T![.],
    /// The await token.
    pub await_token: T![await],
}

expr_parse!(ExprAwait, ".await expression");
