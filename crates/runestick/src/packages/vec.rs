//! Package containing array functions.

use crate::context::{ContextError, Module};
use crate::value::Value;

/// Get the module for the array package.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new(&["std", "vec"]);

    module.ty(&["Vec"]).build::<Vec<Value>>()?;
    module.inst_fn("len", Vec::<Value>::len)?;
    Ok(module)
}
