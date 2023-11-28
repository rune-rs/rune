use std::path::Path;

use crate::ast::Span;
use crate::compile::WithSpan;
use crate::workspace::WorkspaceError;
use crate::Source;

use super::WorkspaceErrorKind;

/// A source loader.
pub trait SourceLoader {
    /// Load the given path.
    fn load(&mut self, span: Span, path: &Path) -> Result<Source, WorkspaceError>;
}

/// A filesystem-based source loader.
#[derive(Default)]
pub struct FileSourceLoader {}

impl FileSourceLoader {
    /// Construct a new filesystem-based source loader.
    pub fn new() -> Self {
        Self::default()
    }
}

impl SourceLoader for FileSourceLoader {
    fn load(&mut self, span: Span, path: &Path) -> Result<Source, WorkspaceError> {
        match Source::from_path(path) {
            Ok(source) => Ok(source),
            Err(error) => Err(WorkspaceError::new(
                span,
                WorkspaceErrorKind::Source {
                    path: path.try_into().with_span(span)?,
                    error,
                },
            )),
        }
    }
}
