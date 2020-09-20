use crate::ast;
use crate::{Spanned, ToTokens};

/// A statement within a block.
#[derive(Debug, Clone, ToTokens, Spanned)]
pub enum Stmt {
    /// A declaration.
    Item(ast::Item),
    /// An expression.
    Expr(ast::Expr),
    /// An expression followed by a semicolon.
    Semi(ast::Expr, ast::SemiColon),
}
