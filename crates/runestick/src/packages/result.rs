//! The `core` package.
//!
//! Contains functions such as:
//! * `dbg` to debug print to stdout.

use crate::{ContextError, Module, Value};

fn is_ok(result: &Result<Value, Value>) -> bool {
    result.is_ok()
}

fn is_err(result: &Result<Value, Value>) -> bool {
    result.is_err()
}

/// Install the core package into the given functions namespace.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new(&["std", "result"]);
    module.result(&["Result"])?;
    module.inst_fn("is_ok", is_ok)?;
    module.inst_fn("is_err", is_err)?;
    Ok(module)
}
