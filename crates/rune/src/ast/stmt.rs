use crate::ast;
use crate::{Ast, Spanned};

/// A statement within a block.
#[derive(Debug, Clone, Ast, Spanned)]
pub enum Stmt {
    /// A declaration.
    Item(ast::Item),
    /// An expression.
    Expr(ast::Expr),
    /// An expression followed by a semicolon.
    Semi(ast::Expr, ast::SemiColon),
}
