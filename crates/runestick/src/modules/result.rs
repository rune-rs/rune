//! The `std::result` module.

use crate::{ContextError, Function, Module, Value, VmError};

/// Construct the `std::result` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", &["result"]);
    // Sorted for ease of finding
    module.result(&["Result"])?;
    module.inst_fn("is_ok", is_ok)?;
    module.inst_fn("is_err", is_err)?;
    module.inst_fn("unwrap", unwrap_impl)?;
    module.inst_fn("expect", expect_impl)?;
    module.inst_fn("and_then", and_then_impl)?;
    module.inst_fn("map", map_impl)?;
    Ok(module)
}

fn is_ok(result: &Result<Value, Value>) -> bool {
    result.is_ok()
}

fn is_err(result: &Result<Value, Value>) -> bool {
    result.is_err()
}

fn unwrap_impl(result: Result<Value, Value>) -> Result<Value, VmError> {
    result.map_err(|err| {
        VmError::panic(format!(
            "called `Result::unwrap()` on an `Err` value: {:?}",
            err
        ))
    })
}

fn expect_impl(result: Result<Value, Value>, message: &str) -> Result<Value, VmError> {
    result.map_err(|err| VmError::panic(format!("{}: {:?}", message, err)))
}

fn and_then_impl(
    this: &Result<Value, Value>,
    then: Function,
) -> Result<Result<Value, Value>, VmError> {
    match this {
        Ok(v) => Ok(then.call::<_, _>((v,))?),
        Err(e) => Ok(Err(e.clone())),
    }
}

fn map_impl(this: &Result<Value, Value>, then: Function) -> Result<Result<Value, Value>, VmError> {
    match this {
        Ok(v) => Ok(Ok(then.call::<_, _>((v,))?)),
        Err(e) => Ok(Err(e.clone())),
    }
}
