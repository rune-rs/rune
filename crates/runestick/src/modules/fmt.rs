//! The `std::fmt` module.

use crate::{ContextError, Module, Protocol};
use std::fmt;
use std::fmt::Write as _;

/// Construct the `std::fmt` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate("std", &["fmt"]);
    module.ty::<std::fmt::Error>()?;
    module.inst_fn(Protocol::STRING_DISPLAY, format_fmt_error)?;

    module.ty::<crate::Format>()?;
    Ok(module)
}

fn format_fmt_error(error: &std::fmt::Error, buf: &mut String) -> fmt::Result {
    write!(buf, "{}", error)
}
