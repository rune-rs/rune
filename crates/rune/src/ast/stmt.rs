use crate::ast;
use crate::{Ast, Spanned};
use runestick::Span;

/// A statement within a block.
#[derive(Debug, Clone, Ast)]
pub enum Stmt {
    /// A declaration.
    Item(ast::Item),
    /// An expression.
    Expr(ast::Expr),
    /// An expression followed by a semicolon.
    Semi(ast::Expr, ast::SemiColon),
}

impl Spanned for Stmt {
    fn span(&self) -> Span {
        match self {
            Self::Item(decl) => decl.span(),
            Self::Expr(expr) => expr.span(),
            Self::Semi(expr, semi) => expr.span().join(semi.span()),
        }
    }
}
