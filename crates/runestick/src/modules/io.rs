//! The `std::io` module.

use crate::{ContextError, Module};
use std::fmt;
use std::fmt::Write as _;

/// Construct the `std::io` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new(&["std", "io"]);
    module.ty::<std::io::Error>()?;
    module.inst_fn(crate::STRING_DISPLAY, format_io_error)?;
    Ok(module)
}

fn format_io_error(error: &std::io::Error, buf: &mut String) -> fmt::Result {
    write!(buf, "{}", error)
}
