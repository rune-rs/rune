use crate::no_std::path::Path;
use crate::no_std::prelude::*;

use crate::ast::Span;
use crate::compile::{self, CompileErrorKind, ComponentRef, Item};
use crate::Source;

/// A source loader.
pub trait SourceLoader {
    /// Load the given URL.
    fn load(&mut self, root: &Path, item: &Item, span: Span) -> compile::Result<Source>;
}

/// A source loader which does not support loading anything and will error.
#[derive(Default)]
#[non_exhaustive]
pub struct NoopSourceLoader;

impl SourceLoader for NoopSourceLoader {
    fn load(&mut self, _: &Path, _: &Item, span: Span) -> compile::Result<Source> {
        Err(compile::Error::msg(span, "File loading is not supported"))
    }
}

/// A filesystem-based source loader.
#[derive(Default)]
#[non_exhaustive]
pub struct FileSourceLoader;

impl FileSourceLoader {
    /// Construct a new filesystem-based source loader.
    pub fn new() -> Self {
        Self::default()
    }
}

impl SourceLoader for FileSourceLoader {
    fn load(&mut self, root: &Path, item: &Item, span: Span) -> compile::Result<Source> {
        let mut base = root.to_owned();

        if !base.pop() {
            return Err(compile::Error::new(
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
                return Err(compile::Error::new(
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
                return Err(compile::Error::new(
                    span,
                    CompileErrorKind::ModNotFound { path: base },
                ));
            }
        };

        match Source::from_path(path) {
            Ok(source) => Ok(source),
            Err(error) => Err(compile::Error::new(
                span,
                CompileErrorKind::FileError {
                    path: path.to_owned(),
                    error,
                },
            )),
        }
    }
}
