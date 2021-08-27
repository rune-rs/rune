//! The `std::option` module.

use crate::{ContextError, Function, Module, Protocol, Shared, Value, VmError};

/// Construct the `std::option` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", &["option"]);
    module.option(&["Option"])?;
    // Sorted for ease of finding
    module.inst_fn("and_then", and_then_impl)?;
    module.inst_fn("expect", expect_impl)?;
    module.inst_fn("is_none", Option::<Value>::is_none)?;
    module.inst_fn("is_some", Option::<Value>::is_some)?;
    module.inst_fn("iter", option_iter)?;
    module.inst_fn("map", map_impl)?;
    module.inst_fn("take", take_impl)?;
    module.inst_fn("transpose", transpose_impl)?;
    module.inst_fn("unwrap", unwrap_impl)?;
    module.inst_fn("unwrap_or", Option::<Value>::unwrap_or)?;
    module.inst_fn("unwrap_or_else", unwrap_or_else_impl)?;
    module.inst_fn(Protocol::INTO_ITER, option_iter)?;
    Ok(module)
}

fn unwrap_or_else_impl(this: &Option<Value>, default: Function) -> Result<Value, VmError> {
    if let Some(this) = this {
        return Ok(this.clone());
    }

    default.call(())
}

/// Transpose functions, translates an Option<Result<T, E>> into a `Result<Option<T>, E>`.
fn transpose_impl(this: &Option<Value>) -> Result<Value, VmError> {
    Ok(Value::from(Shared::new(match this.clone() {
        Some(some) => match some.into_result()?.borrow_ref()?.clone() {
            Ok(ok) => Ok(Value::from(Shared::new(Some(ok)))),
            Err(err) => Err(err),
        },
        None => Ok(Value::from(Shared::new(None::<Value>))),
    })))
}

fn option_iter(option: &Option<Value>) -> crate::Iterator {
    crate::Iterator::from_double_ended("std::option::Iter", option.clone().into_iter())
}

fn unwrap_impl(option: Option<Value>) -> Result<Value, VmError> {
    option.ok_or_else(|| VmError::panic("called `Option::unwrap()` on a `None` value"))
}

fn expect_impl(option: Option<Value>, message: &str) -> Result<Value, VmError> {
    option.ok_or_else(|| VmError::panic(message.to_owned()))
}

fn map_impl(option: &Option<Value>, then: Function) -> Result<Option<Value>, VmError> {
    match option {
        // no need to clone v, passing the same reference forward
        Some(v) => then.call::<_, _>((v,)).map(Some),
        None => Ok(None),
    }
}

fn and_then_impl(option: &Option<Value>, then: Function) -> Result<Option<Value>, VmError> {
    match option {
        // no need to clone v, passing the same reference forward
        Some(v) => then.call::<_, _>((v,)),
        None => Ok(None),
    }
}

fn take_impl(option: &mut Option<Value>) -> Option<Value> {
    option.take()
}
