//! Package containing array functions.

use crate::context::{ContextError, Module};
use crate::value::{Array, ValuePtr};

/// Get the module for the array package.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new(&["std", "array"]);

    module.ty(&["Array"]).build::<Array<ValuePtr>>()?;
    module.inst_fn("len", Array::<ValuePtr>::len)?;
    Ok(module)
}
