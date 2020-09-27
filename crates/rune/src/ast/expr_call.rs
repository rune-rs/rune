use crate::ast;
use crate::{Parse, Spanned, ToTokens};

/// A function call `<expr>(<args>)`.
///
/// # Examples
///
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::Expr>("test()");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Parse, ToTokens, Spanned)]
pub struct ExprCall {
    /// Attributes associated with expression.
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The name of the function being called.
    pub expr: Box<ast::Expr>,
    /// The arguments of the function call.
    pub args: ast::Parenthesized<ast::Expr, ast::Comma>,
}
