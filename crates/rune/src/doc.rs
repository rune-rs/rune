//! Helper to generate documentation from a context.

mod context;
pub(crate) use self::context::Context;

mod templating;

mod html;
pub use self::html::write_html;

mod visitor;
pub use self::visitor::Visitor;
pub(crate) use self::visitor::VisitorData;
