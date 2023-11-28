//! Types for dealing with workspaces of rune code.

/// The name of the toplevel manifest `Rune.toml`.
pub const MANIFEST_FILE: &str = "Rune.toml";

mod glob;

mod spanned_value;

mod build;
pub use self::build::{prepare, Build, BuildError};

cfg_emit! {
    mod emit;
    #[doc(inline)]
    pub use self::emit::EmitError;
}

mod error;
pub use self::error::WorkspaceError;
pub(crate) use self::error::WorkspaceErrorKind;

mod manifest;
pub use self::manifest::{Found, FoundKind, FoundPackage, Manifest, Package, WorkspaceFilter};

mod diagnostics;
pub use self::diagnostics::{Diagnostic, Diagnostics, FatalDiagnostic};

mod source_loader;
pub use self::source_loader::{FileSourceLoader, SourceLoader};
