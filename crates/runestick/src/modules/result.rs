//! The `std::result` module.

use crate::{ContextError, Module, Value};

/// Construct the `std::result` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new(&["std", "result"]);
    module.result(&["Result"])?;
    module.inst_fn("is_ok", is_ok)?;
    module.inst_fn("is_err", is_err)?;
    Ok(module)
}

fn is_ok(result: &Result<Value, Value>) -> bool {
    result.is_ok()
}

fn is_err(result: &Result<Value, Value>) -> bool {
    result.is_err()
}
