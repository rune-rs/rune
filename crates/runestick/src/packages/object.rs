//! Package containing object functions.

use crate::{ContextError, Module, Object, Value};

/// Get the module for the object package.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new(&["std", "object"]);

    module.ty(&["Object"]).build::<Object<Value>>()?;
    module.inst_fn("len", Object::<Value>::len)?;
    Ok(module)
}
