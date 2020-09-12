//! The `std::fmt` module.

use crate::{ContextError, Module};
use std::fmt;
use std::fmt::Write as _;

/// Construct the `std::fmt` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new(&["std", "fmt"]);
    module.ty::<std::fmt::Error>()?;
    module.inst_fn(crate::STRING_DISPLAY, format_fmt_error)?;
    Ok(module)
}

fn format_fmt_error(error: &std::fmt::Error, buf: &mut String) -> fmt::Result {
    write!(buf, "{}", error)
}
