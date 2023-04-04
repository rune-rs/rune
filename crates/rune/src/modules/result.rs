//! The `std::result` module.

use crate::runtime::{Function, Value, VmError, VmResult};
use crate::{ContextError, Module};

/// Construct the `std::result` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", ["result"]);
    // Sorted for ease of finding
    module.result(["Result"])?;
    module.inst_fn("ok", ok)?;
    module.inst_fn("is_ok", is_ok)?;
    module.inst_fn("is_err", is_err)?;
    module.inst_fn("unwrap", unwrap_impl)?;
    module.inst_fn("unwrap_or", Result::<Value, Value>::unwrap_or)?;
    module.inst_fn("expect", expect_impl)?;
    module.inst_fn("and_then", and_then_impl)?;
    module.inst_fn("map", map_impl)?;
    Ok(module)
}

fn ok(result: &Result<Value, Value>) -> Option<Value> {
    result.as_ref().ok().cloned()
}

fn is_ok(result: &Result<Value, Value>) -> bool {
    result.is_ok()
}

fn is_err(result: &Result<Value, Value>) -> bool {
    result.is_err()
}

fn unwrap_impl(result: Result<Value, Value>) -> VmResult<Value> {
    match result {
        Ok(value) => VmResult::Ok(value),
        Err(err) => VmResult::err(VmError::panic(format!(
            "called `Result::unwrap()` on an `Err` value: {:?}",
            err
        ))),
    }
}

fn expect_impl(result: Result<Value, Value>, message: &str) -> VmResult<Value> {
    match result {
        Ok(value) => VmResult::Ok(value),
        Err(err) => VmResult::err(VmError::panic(format!("{}: {:?}", message, err))),
    }
}

fn and_then_impl(this: &Result<Value, Value>, then: Function) -> VmResult<Result<Value, Value>> {
    match this {
        // No need to clone v, passing the same reference forward
        Ok(v) => VmResult::Ok(vm_try!(then.call::<_, _>((v,)))),
        Err(e) => VmResult::Ok(Err(e.clone())),
    }
}

fn map_impl(this: &Result<Value, Value>, then: Function) -> VmResult<Result<Value, Value>> {
    match this {
        // No need to clone v, passing the same reference forward
        Ok(v) => VmResult::Ok(Ok(vm_try!(then.call::<_, _>((v,))))),
        Err(e) => VmResult::Ok(Err(e.clone())),
    }
}
