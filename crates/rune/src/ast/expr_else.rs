use crate::ast;
use crate::{Ast, Parse, Spanned};
use runestick::Span;

/// An else branch of an if expression.
#[derive(Debug, Clone, Ast, Parse)]
pub struct ExprElse {
    /// The `else` token.
    pub else_: ast::Else,
    /// The body of the else statement.
    pub block: Box<ast::ExprBlock>,
}

impl Spanned for ExprElse {
    fn span(&self) -> Span {
        self.else_.span().join(self.block.span())
    }
}
