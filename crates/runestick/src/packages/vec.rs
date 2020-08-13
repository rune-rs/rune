//! Package containing array functions.

use crate::context::{ContextError, Module};
use crate::value::Value;

/// Get the module for the array package.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new(&["std", "vec"]);

    module.ty(&["Vec"]).build::<Vec<Value>>()?;
    module.function(&["Vec", "new"], Vec::<Value>::new)?;
    module.inst_fn("len", Vec::<Value>::len)?;
    module.inst_fn("push", Vec::<Value>::push)?;
    module.inst_fn("clear", Vec::<Value>::clear)?;
    module.inst_fn("pop", Vec::<Value>::pop)?;
    Ok(module)
}
