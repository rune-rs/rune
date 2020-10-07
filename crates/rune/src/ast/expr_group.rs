use crate::ast;
use crate::{Spanned, ToTokens};

/// A prioritized expression group `(<expr>)`.
///
/// # Examples
///
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::ExprGroup>("(for i in x {})");
/// testing::roundtrip::<ast::ExprGroup>("(1 + 2)");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub struct ExprGroup {
    /// Attributes associated with expression.
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The open parenthesis.
    pub open: ast::OpenParen,
    /// The grouped expression.
    pub expr: ast::Expr,
    /// The close parenthesis.
    pub close: ast::CloseParen,
}

expr_parse!(ExprGroup, "group expression");
