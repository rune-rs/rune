use crate::ast;
use crate::{Parse, Spanned, ToTokens};

/// A prioritized expression group `(<expr>)`.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Parse, Spanned)]
pub struct ExprGroup {
    /// Attributes associated with expression.
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The open parenthesis.
    pub open: ast::OpenParen,
    /// The grouped expression.
    pub expr: Box<ast::Expr>,
    /// The close parenthesis.
    pub close: ast::CloseParen,
}
