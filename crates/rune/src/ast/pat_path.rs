use crate::ast;
use crate::{Spanned, ToTokens};

/// A tuple pattern.
#[derive(Debug, Clone, ToTokens, Spanned)]
pub struct PatPath {
    /// The path, if the tuple is typed.
    pub path: ast::Path,
}
