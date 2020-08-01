//! Package containing object functions.

use crate::context::{ContextError, Module};
use crate::value::{Object, ValuePtr};

/// Get the module for the object package.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new(&["std", "object"]);

    module.ty(&["Object"]).build::<Object<ValuePtr>>()?;
    module.inst_fn("len", Object::<ValuePtr>::len)?;
    Ok(module)
}
