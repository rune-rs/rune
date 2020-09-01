//! The `core` package.
//!
//! Contains functions such as:
//! * `dbg` to debug print to stdout.

use crate::{ContextError, FnPtr, Module, Value, VmError};

async fn unwrap_or_else_impl(this: &Option<Value>, default: FnPtr) -> Result<Value, VmError> {
    if let Some(this) = this {
        return Ok(this.clone());
    }

    Ok(default.call(()).await?)
}

/// Install the core package into the given functions namespace.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new(&["std", "option"]);
    module.option(&["Option"])?;
    module.inst_fn("unwrap_or", Option::<Value>::unwrap_or)?;
    module.async_inst_fn("unwrap_or_else", unwrap_or_else_impl)?;
    Ok(module)
}
