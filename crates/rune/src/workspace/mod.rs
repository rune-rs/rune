//! Types for dealing with workspaces of rune code.

/// The name of the toplevel manifest `Rune.toml`.
pub const MANIFEST_FILE: &str = "Rune.toml";

mod glob;

mod spanned_value;

mod build;
pub use self::build::{prepare, Build, BuildError};

#[cfg(feature = "emit")]
#[cfg_attr(rune_docsrs, doc(cfg(feature = "emit")))]
mod emit;
#[cfg(feature = "emit")]
#[cfg_attr(rune_docsrs, doc(cfg(feature = "emit")))]
#[doc(inline)]
pub use self::emit::EmitError;

mod error;
pub use self::error::WorkspaceError;
pub(crate) use self::error::WorkspaceErrorKind;

mod manifest;
pub use self::manifest::{
    Found, FoundKind, FoundPackage, Loader as ManifestLoader, Manifest, Package, WorkspaceFilter,
};

mod diagnostics;
pub use self::diagnostics::{Diagnostic, Diagnostics, FatalDiagnostic};

mod source_loader;
pub use self::source_loader::{FileSourceLoader, SourceLoader};
