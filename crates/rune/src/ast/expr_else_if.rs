use crate::ast;
use crate::{Parse, Spanned, ToTokens};

/// An else branch of an if expression.
#[derive(Debug, Clone, ToTokens, Parse, Spanned)]
pub struct ExprElseIf {
    /// The `else` token.
    pub else_: ast::Else,
    /// The `if` token.
    pub if_: ast::If,
    /// The condition for the branch.
    pub condition: ast::Condition,
    /// The body of the else statement.
    pub block: Box<ast::ExprBlock>,
}
