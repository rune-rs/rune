#[cfg(feature = "std")]
use crate::alloc::prelude::*;
use crate::ast::Spanned;
use crate::compile;
#[cfg(feature = "std")]
use crate::compile::ErrorKind;
#[cfg(feature = "std")]
use crate::item::ComponentRef;
use crate::{Item, Source, SourceId, Sources};

/// A source loader.
pub trait SourceLoader {
    /// Load the given URL.
    fn load(
        &mut self,
        sources: &Sources,
        id: SourceId,
        item: &Item,
        span: &dyn Spanned,
    ) -> compile::Result<Source>;
}

/// A source loader which does not support loading anything and will error.
#[derive(Default)]
#[non_exhaustive]
pub struct NoopSourceLoader;

impl SourceLoader for NoopSourceLoader {
    fn load(
        &mut self,
        _: &Sources,
        _: SourceId,
        _: &Item,
        span: &dyn Spanned,
    ) -> compile::Result<Source> {
        Err(compile::Error::msg(span, "Source loading is not supported"))
    }
}

/// A filesystem-based source loader.
#[derive(Default)]
#[non_exhaustive]
#[cfg(feature = "std")]
#[cfg_attr(rune_docsrs, doc(cfg(feature = "std")))]
pub struct FileSourceLoader;

#[cfg(feature = "std")]
#[cfg_attr(rune_docsrs, doc(cfg(feature = "std")))]
impl FileSourceLoader {
    /// Construct a new filesystem-based source loader.
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(feature = "std")]
#[cfg_attr(rune_docsrs, doc(cfg(feature = "std")))]
impl SourceLoader for FileSourceLoader {
    fn load(
        &mut self,
        sources: &Sources,
        id: SourceId,
        item: &Item,
        span: &dyn Spanned,
    ) -> compile::Result<Source> {
        let Some(base) = sources.path(id) else {
            return Err(compile::Error::new(span, ErrorKind::SourceWithoutPath));
        };

        let mut base = base.try_to_owned()?;

        if !base.pop() {
            return Err(compile::Error::new(
                span,
                ErrorKind::UnsupportedModuleRoot {
                    root: base.try_to_owned()?,
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
