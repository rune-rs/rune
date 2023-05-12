//! The `std::result` module.

use crate as rune;
use crate::runtime::{Function, Panic, Value, VmResult};
use crate::{ContextError, Module};

/// Construct the `std::result` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", ["result"]);
    // Sorted for ease of finding
    let mut result = module
        .result(["Result"])?
        .static_docs(&["Result is a type that represents either success (Ok) or failure (Err)."]);

    result
        .variant_mut(0)?
        .static_docs(&["Contains the success value"]);

    result
        .variant_mut(1)?
        .static_docs(&["Contains the error value"]);

    module.function_meta(ok)?;
    module.associated_function("is_ok", is_ok)?;
    module.associated_function("is_err", is_err)?;
    module.associated_function("unwrap", unwrap_impl)?;
    module.associated_function("unwrap_or", Result::<Value, Value>::unwrap_or)?;
    module.associated_function("expect", expect_impl)?;
    module.associated_function("and_then", and_then_impl)?;
    module.associated_function("map", map_impl)?;
    Ok(module)
}

/// Converts from `Result<T, E>` to `Option<T>`.
///
/// Converts self into an `Option<T>`, consuming `self`, and discarding the
/// error, if any.
#[rune::function(instance)]
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
        Err(err) => VmResult::err(Panic::msg(format_args!(
            "called `Result::unwrap()` on an `Err` value: {:?}",
            err
        ))),
    }
}

fn expect_impl(result: Result<Value, Value>, message: &str) -> VmResult<Value> {
    match result {
        Ok(value) => VmResult::Ok(value),
        Err(err) => VmResult::err(Panic::msg(format_args!("{}: {:?}", message, err))),
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
