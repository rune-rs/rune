//! The `std::test` module.

use crate::{ContextError, Module, Panic};
use std::fmt;

/// Construct the `std::test` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new(&["std", "test"]);
    module.function(&["assert"], assert)?;
    Ok(module)
}

#[derive(Debug)]
struct AssertionFailed(String);

impl fmt::Display for AssertionFailed {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "assertion failed `{}`", self.0)
    }
}

/// Assert that a value is true.
fn assert(value: bool, message: &str) -> Result<(), Panic> {
    if !value {
        return Err(Panic::custom(AssertionFailed(message.to_string())));
    }

    Ok(())
}
