use crate::ast;
use std::fmt;

/// Information on the visibility of an item.
#[derive(Debug, Clone, Copy)]
pub enum Visibility {
    /// Inherited, or private visibility.
    Inherited,
    /// Something that is publicly visible `pub`.
    Public,
    /// Something that is only visible in the current crate `pub(crate)`.
    Crate,
}

impl Visibility {
    pub fn from_ast(vis: &ast::Visibility) -> Self {
        match vis {
            ast::Visibility::Inherited => Self::Inherited,
            ast::Visibility::Public(_) => Self::Public,
            // TODO: restricted means more than just `crate`.
            ast::Visibility::Restricted(_) => Self::Crate,
        }
    }
}

impl fmt::Display for Visibility {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Visibility::Inherited => write!(f, "private")?,
            Visibility::Public => write!(f, "pub")?,
            Visibility::Crate => write!(f, "pub(crate)")?,
        }

        Ok(())
    }
}
