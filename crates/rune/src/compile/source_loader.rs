use std::path::Path;

use crate::ast::Span;
use crate::compile::{CompileError, CompileErrorKind, ComponentRef, Item};
use crate::Source;

/// A source loader.
pub trait SourceLoader {
    /// Load the given URL.
    fn load(&mut self, root: &Path, item: &Item, span: Span) -> Result<Source, CompileError>;
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
    fn load(&mut self, root: &Path, item: &Item, span: Span) -> Result<Source, CompileError> {
        let mut base = root.to_owned();

        if !base.pop() {
            return Err(CompileError::new(
                span,
                CompileErrorKind::UnsupportedModuleRoot {
                    root: root.to_owned(),
                },
            ));
        }

        for c in item {
            if let ComponentRef::Str(string) = c {
                base.push(string);
            } else {
                return Err(CompileError::new(
                    span,
                    CompileErrorKind::UnsupportedModuleItem {
                        item: item.to_owned(),
                    },
                ));
            }
        }

        let candidates = [base.join("mod.rn"), base.with_extension("rn")];

        let mut found = None;

        for path in &candidates[..] {
            if path.is_file() {
                found = Some(path);
                break;
            }
        }

        let path = match found {
            Some(path) => path,
            None => {
                return Err(CompileError::new(
                    span,
                    CompileErrorKind::ModNotFound { path: base },
                ));
            }
        };

        match Source::from_path(path) {
            Ok(source) => Ok(source),
            Err(error) => Err(CompileError::new(
                span,
                CompileErrorKind::FileError {
                    path: path.to_owned(),
                    error,
                },
            )),
        }
    }
}
