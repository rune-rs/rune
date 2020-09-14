use crate::CompileError;
use runestick::{Source, Span, Url};

/// A source loader.
pub trait SourceLoader {
    /// Load the given URL.
    fn load(&mut self, url: &Url, name: &str, span: Span) -> Result<Source, CompileError>;
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
    fn load(&mut self, url: &Url, name: &str, span: Span) -> Result<Source, CompileError> {
        if url.scheme() != "file" {
            return Err(CompileError::UnsupportedLoadUrl {
                span,
                url: url.clone(),
            });
        }

        let path = url
            .to_file_path()
            .map_err(|_| CompileError::UnsupportedLoadUrl {
                span,
                url: url.clone(),
            })?;

        let base = match path.parent() {
            Some(parent) => parent.join(name),
            None => {
                return Err(CompileError::UnsupportedFileMod { span });
            }
        };

        let candidates = [
            base.join("mod").with_extension("rn"),
            base.with_extension("rn"),
        ];

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
                return Err(CompileError::ModNotFound { path: base, span });
            }
        };

        match Source::from_path(path) {
            Ok(source) => Ok(source),
            Err(error) => {
                return Err(CompileError::ModFileError {
                    span,
                    path: path.to_owned(),
                    error,
                });
            }
        }
    }
}
