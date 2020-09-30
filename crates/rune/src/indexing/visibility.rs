use crate::ast;
use crate::{CompileError, CompileErrorKind, Spanned as _};
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
    pub fn from_ast(vis: &ast::Visibility) -> Result<Self, CompileError> {
        let span = match vis {
            ast::Visibility::Inherited => return Ok(Self::Inherited),
            ast::Visibility::Public(_) => return Ok(Self::Public),
            ast::Visibility::Crate(_) => return Ok(Self::Crate),
            ast::Visibility::Super(restrict) => restrict.span(),
            ast::Visibility::SelfValue(restrict) => restrict.span(),
            ast::Visibility::In(restrict) => restrict.span(),
        };

        Err(CompileError::new(
            span,
            CompileErrorKind::UnsupportedVisibility,
        ))
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
