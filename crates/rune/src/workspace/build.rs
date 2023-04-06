use thiserror::Error;

use crate::Sources;
use crate::workspace::{SourceLoader, Diagnostics, FileSourceLoader};
use crate::workspace::manifest::{Loader, Manifest};

/// Failed to build workspace.
#[derive(Debug, Error)]
#[error("Failed to load workspace (see diagnostics for details)")]
pub struct BuildError;

/// Prepare a workspace build.
pub fn prepare(sources: &mut Sources) -> Build<'_> {
    Build {
        sources,
        diagnostics: None,
        source_loader: None,
    }
}

/// A prepared build.
pub struct Build<'a> {
    sources: &'a mut Sources,
    diagnostics: Option<&'a mut Diagnostics>,
    source_loader: Option<&'a mut dyn SourceLoader>,
}

impl<'a> Build<'a> {
    /// Associate a specific diagnostic with the build.
    pub fn with_diagnostics(self, diagnostics: &'a mut Diagnostics) -> Self {
        Self {
            diagnostics: Some(diagnostics),
            ..self
        }
    }

    /// Associate a specific source loader with the build.
    ///
    /// By default [`FileSourceLoader`] will be used.
    pub fn with_source_loader(self, source_loader: &'a mut dyn SourceLoader) -> Self {
        Self {
            source_loader: Some(source_loader),
            ..self
        }
    }

    /// Perform the build.
    pub fn build(self) -> Result<Manifest, BuildError> {
        let mut diagnostics;

        let diagnostics = match self.diagnostics {
            Some(diagnostics) => diagnostics,
            None => {
                diagnostics = Diagnostics::new();
                &mut diagnostics
            }
        };

        let mut source_loader;

        let source_loader = match self.source_loader {
            Some(source_loader) => source_loader,
            None => {
                source_loader = FileSourceLoader::new();
                &mut source_loader
            }
        };

        let mut manifest = Manifest::default();

        for id in self.sources.source_ids() {
            let mut loader = Loader::new(id, self.sources, diagnostics, source_loader, &mut manifest);
            loader.load_manifest();
        }

        if diagnostics.has_errors() {
            return Err(BuildError);
        }
    
        Ok(manifest)
    }
}
