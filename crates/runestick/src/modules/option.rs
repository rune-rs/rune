//! The `std::option` module.

use crate::{ContextError, Function, Module, Protocol, Shared, Value, VmError};

/// Construct the `std::option` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate("std", &["option"]);
    module.option(&["Option"])?;
    module.inst_fn("unwrap_or", Option::<Value>::unwrap_or)?;
    module.inst_fn("is_none", Option::<Value>::is_none)?;
    module.inst_fn("is_some", Option::<Value>::is_some)?;
    module.inst_fn("unwrap_or_else", unwrap_or_else_impl)?;
    module.inst_fn("transpose", transpose_impl)?;
    module.inst_fn("iter", option_iter)?;
    module.inst_fn(Protocol::INTO_ITER, option_iter)?;
    Ok(module)
}

fn unwrap_or_else_impl(this: &Option<Value>, default: Function) -> Result<Value, VmError> {
    if let Some(this) = this {
        return Ok(this.clone());
    }

    Ok(default.call(())?)
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
