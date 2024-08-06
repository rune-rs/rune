//! Helper to generate documentation from a context.

#[cfg(feature = "cli")]
mod context;
#[cfg(feature = "cli")]
pub(crate) use self::context::{Context, Function, Kind, Meta, Signature};

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

#[cfg(feature = "languageserver")]
mod visitor;
#[cfg(feature = "languageserver")]
pub(crate) use self::visitor::{Visitor, VisitorData};

#[cfg(feature = "cli")]
pub(crate) mod markdown;
