//! Types for dealing with workspaces of rune code.

/// The name of the toplevel manifest `Rune.toml`.
pub const MANIFEST_FILE: &str = "Rune.toml";

mod spanned_value;

mod build;
pub use self::build::{prepare, Build, BuildError};

cfg_emit! {
    mod emit;
    #[doc(inline)]
    pub use self::emit::EmitError;
}

mod error;

pub use self::error::{WorkspaceErrorKind, WorkspaceError};

mod manifest;
pub use self::manifest::{Manifest, WorkspaceFilter};

mod diagnostics;
pub use self::diagnostics::{Diagnostics};
pub(crate) use self::diagnostics::{Diagnostic, FatalDiagnostic};
