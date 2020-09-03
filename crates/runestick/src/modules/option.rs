//! The `std::option` module.

/// Construct the `std::option` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new(&["std", "option"]);
    module.option(&["Option"])?;
    module.inst_fn("unwrap_or", Option::<Value>::unwrap_or)?;
    module.inst_fn("is_none", Option::<Value>::is_none)?;
    module.inst_fn("is_some", Option::<Value>::is_some)?;
    module.async_inst_fn("unwrap_or_else", unwrap_or_else_impl)?;
    Ok(module)
}

use crate::{ContextError, FnPtr, Module, Value, VmError};

async fn unwrap_or_else_impl(this: &Option<Value>, default: FnPtr) -> Result<Value, VmError> {
    if let Some(this) = this {
        return Ok(this.clone());
    }

    Ok(default.call(()).await?)
}
