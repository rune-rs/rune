use crate::{CompileError, CompileErrorKind};
use runestick::{Component, Item, Source, Span, Url};

/// A source loader.
pub trait SourceLoader {
    /// Load the given URL.
    fn load(&mut self, root: &Url, item: &Item, span: Span) -> Result<Source, CompileError>;
}

/// A filesystem-based source loader.
pub struct FileSourceLoader {}

impl FileSourceLoader {
    /// Construct a new filesystem-based source loader.
    pub fn new() -> Self {
        Self {}
    }
}

impl SourceLoader for FileSourceLoader {
    fn load(&mut self, root: &Url, item: &Item, span: Span) -> Result<Source, CompileError> {
        if root.scheme() != "file" {
            return Err(CompileError::new(
                span,
                CompileErrorKind::UnsupportedModuleRoot { url: root.clone() },
            ));
        }

        let mut base = root.to_file_path().map_err(|_| {
            CompileError::new(
                span,
                CompileErrorKind::UnsupportedModuleRoot { url: root.clone() },
            )
        })?;

        if !base.pop() {
            return Err(CompileError::new(
                span,
                CompileErrorKind::UnsupportedModuleRoot { url: root.clone() },
            ));
        }

        for c in item {
            if let Component::String(string) = c {
                base.push(string.as_ref());
            } else {
                return Err(CompileError::new(
                    span,
                    CompileErrorKind::UnsupportedModuleItem { item: item.clone() },
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
            Err(error) => {
                return Err(CompileError::new(
                    span,
                    CompileErrorKind::ModFileError {
                        path: path.to_owned(),
                        error,
                    },
                ));
            }
        }
    }
}
