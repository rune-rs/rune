//! The `std::option` module.

/// Construct the `std::option` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new(&["std", "option"]);
    module.option(&["Option"])?;
    module.inst_fn("unwrap_or", Option::<Value>::unwrap_or)?;
    module.inst_fn("is_none", Option::<Value>::is_none)?;
    module.inst_fn("is_some", Option::<Value>::is_some)?;
    module.inst_fn("unwrap_or_else", unwrap_or_else_impl)?;
    module.inst_fn("transpose", transpose_impl)?;
    Ok(module)
}

use crate::{ContextError, Function, Module, Shared, Value, VmError};

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
