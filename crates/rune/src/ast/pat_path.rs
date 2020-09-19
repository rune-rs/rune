use crate::ast;
use crate::{Ast, Spanned};

/// A tuple pattern.
#[derive(Debug, Clone, Ast, Spanned)]
pub struct PatPath {
    /// The path, if the tuple is typed.
    pub path: ast::Path,
}
