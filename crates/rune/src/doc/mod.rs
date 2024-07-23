//! Helper to generate documentation from a context.

mod context;
use self::context::Context;

mod artifacts;
pub(crate) use self::artifacts::{Artifacts, TestParams};

mod templating;

mod build;
pub(crate) use self::build::build;

mod visitor;
pub(crate) use self::visitor::{Visitor, VisitorData};
