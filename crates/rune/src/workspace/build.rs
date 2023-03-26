use crate::Sources;
use crate::workspace::Diagnostics;
use thiserror::Error;
use crate::workspace::manifest::{self, Loader, Manifest};

/// Failed to build workspace.
#[derive(Debug, Error)]
#[error("failed to load workspace")]
pub struct BuildError;

/// Prepare a workspace build.
pub fn prepare(sources: &mut Sources) -> Build<'_> {
    Build {
        sources,
        diagnostics: None,
    }
}

/// A prepared build.
pub struct Build<'a> {
    sources: &'a mut Sources,
    diagnostics: Option<&'a mut Diagnostics>,
}

impl<'a> Build<'a> {
    /// Associate a specific diagnostic with the build.
    pub fn with_diagnostics(self, diagnostics: &'a mut Diagnostics) -> Self {
        Self {
            diagnostics: Some(diagnostics),
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

        let mut manifest = Manifest::default();

        for id in self.sources.source_ids() {
            let mut l = Loader {
                id,
                sources: self.sources,
                diagnostics,
                manifest: &mut manifest,
            };

            manifest::load_manifest(&mut l);
        }

        if diagnostics.has_errors() {
            return Err(BuildError);
        }
    
        Ok(manifest)
    }
}
