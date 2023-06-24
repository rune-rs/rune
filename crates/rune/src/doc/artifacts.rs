use crate::no_std::path::Path;
use crate::no_std::borrow::Cow;
use crate::no_std::io;
use crate::no_std::prelude::*;

use base64::display::Base64Display;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use relative_path::{RelativePath, RelativePathBuf};
use sha2::{Sha256, Digest};
use anyhow::{Context as _, Error, Result};

/// A collection of artifacts produced by a documentation build.
///
/// This can be disabled through the [`AssetsQueue::disabled`] constructor in
/// case you don't want any static assets to be built.
pub(crate) struct Artifacts {
    enabled: bool,
    assets: Vec<Asset>,
}

impl Artifacts {
    /// Construct a new assets queue.
    pub(crate) fn new() -> Self {
        Self {
            enabled: true,
            assets: Vec::new(),
        }
    }

    /// Build a disabled assets queue.
    pub(crate) fn without_assets() -> Self {
        Self {
            enabled: false,
            assets: Vec::new(),
        }
    }

    /// Iterate over assets produced by this documentation build.
    ///
    /// This is always empty if the [`Artifacts::without_assets`] constructor
    /// was used.
    pub(crate) fn assets(&self) -> impl Iterator<Item = &Asset> {
        self.assets.iter()
    }

    /// Define an asset artifact.
    pub(crate) fn asset<P, F>(
        &mut self,
        path: &P,
        content: F,
    ) -> Result<RelativePathBuf> where P: ?Sized + AsRef<RelativePath>, F: FnOnce() -> Result<Cow<'static, [u8]>> {
        if !self.enabled {
            return Ok(path.as_ref().to_owned())
        }

        let content = content().context("Building asset content")?;

        let mut hasher = Sha256::new();
        hasher.update(content.as_ref());
        let result = hasher.finalize();
        let hash = Base64Display::new(&result[..], &URL_SAFE_NO_PAD);

        let path = path.as_ref();
        let stem = path.file_stem().context("Missing file stem")?;
        let ext = path.extension().context("Missing file extension")?;
        let path = path.with_file_name(format!("{stem}-{hash}.{ext}"));

        self.assets.push(Asset {
            path: path.clone(),
            content,
        });

        Ok(path)
    }
}

/// Asset builder.
pub(crate) struct Asset {
    path: RelativePathBuf,
    content: Cow<'static, [u8]>,
}

impl Asset {
    /// Build the given asset.
    pub(crate) fn build(&self, root: &Path) -> Result<()> {
        let p = self.path.to_path(root);
        tracing::info!("Writing: {}", p.display());
        ensure_parent_dir(&p)?;
        std::fs::write(&p, &self.content).with_context(|| p.display().to_string())?;
        Ok(())
    }
}

/// Ensure parent dir exists.
fn ensure_parent_dir(path: &Path) -> Result<()> {
    if let Some(p) = path.parent() {
        if p.is_dir() {
            return Ok(());
        }

        tracing::info!("create dir: {}", p.display());

        match std::fs::create_dir_all(p) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(Error::from(e)).context(p.display().to_string()),
        }
    }

    Ok(())
}
