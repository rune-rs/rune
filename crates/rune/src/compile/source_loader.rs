use crate::alloc::path::Path;
#[cfg(feature = "std")]
use crate::alloc::prelude::*;
use crate::ast::Spanned;
use crate::compile::{self, Item};
#[cfg(feature = "std")]
use crate::compile::{ComponentRef, ErrorKind};
use crate::Source;

/// A source loader.
pub trait SourceLoader {
    /// Load the given URL.
    fn load(&mut self, root: &Path, item: &Item, span: &dyn Spanned) -> compile::Result<Source>;
}

/// A source loader which does not support loading anything and will error.
#[derive(Default)]
#[non_exhaustive]
pub struct NoopSourceLoader;

impl SourceLoader for NoopSourceLoader {
    fn load(&mut self, _: &Path, _: &Item, span: &dyn Spanned) -> compile::Result<Source> {
        Err(compile::Error::msg(span, "Source loading is not supported"))
    }
}

cfg_std! {
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
        fn load(&mut self, root: &Path, item: &Item, span: &dyn Spanned) -> compile::Result<Source> {
            let mut base = root.try_to_owned()?;

            if !base.pop() {
                return Err(compile::Error::new(
                    span,
                    ErrorKind::UnsupportedModuleRoot {
                        root: root.try_to_owned()?,
                    },
                ));
            }

            for c in item {
                if let ComponentRef::Str(string) = c {
                    base.push(string);
                } else {
                    return Err(compile::Error::new(
                        span,
                        ErrorKind::UnsupportedModuleItem {
                            item: item.try_to_owned()?,
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

            let Some(path) = found else {
                return Err(compile::Error::new(
                    span,
                    ErrorKind::ModNotFound { path: base },
                ));
            };

            match Source::from_path(path) {
                Ok(source) => Ok(source),
                Err(error) => Err(compile::Error::new(
                    span,
                    ErrorKind::SourceError {
                        path: path.clone(),
                        error,
                    },
                )),
            }
        }
    }
}
