use crate::ast;
use crate::{Parse, Spanned, ToTokens};

/// An else branch of an if expression.
#[derive(Debug, Clone, ToTokens, Parse, Spanned)]
pub struct ExprElse {
    /// The `else` token.
    pub else_: ast::Else,
    /// The body of the else statement.
    pub block: Box<ast::ExprBlock>,
}
