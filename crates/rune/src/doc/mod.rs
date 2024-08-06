//! Helper to generate documentation from a context.

#[cfg(feature = "cli")]
mod context;
#[cfg(feature = "cli")]
use self::context::Context;

#[cfg(feature = "cli")]
mod artifacts;
#[cfg(feature = "cli")]
pub(crate) use self::artifacts::{Artifacts, TestParams};

#[cfg(feature = "cli")]
mod templating;

#[cfg(feature = "cli")]
mod build;
#[cfg(feature = "cli")]
pub(crate) use self::build::build;

#[cfg(any(feature = "languageserver", feature = "cli"))]
mod visitor;
#[cfg(any(feature = "languageserver", feature = "cli"))]
pub(crate) use self::visitor::{Visitor, VisitorData};
