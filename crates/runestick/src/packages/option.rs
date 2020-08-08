//! The `core` package.
//!
//! Contains functions such as:
//! * `dbg` to debug print to stdout.

use crate::context::{ContextError, Module};
use crate::value::ValuePtr;

/// Install the core package into the given functions namespace.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new(&["std", "option"]);

    module.ty(&["Option"]).build::<Option<ValuePtr>>()?;

    module.function(&["Option", "Some"], Option::<ValuePtr>::Some)?;
    module.function(&["Option", "None"], || Option::<ValuePtr>::None)?;
    Ok(module)
}
