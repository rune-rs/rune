//! The `std::option` module.

use crate::no_std::prelude::*;

use crate as rune;
use crate::runtime::{Function, Iterator, Panic, Protocol, Shared, Value, VmResult};
use crate::{ContextError, Module};

/// Construct the `std::option` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", ["option"]);
    module.option(["Option"])?;
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
    module.function_meta(unwrap_or_else)?;
    module.inst_fn(Protocol::INTO_ITER, option_iter)?;
    Ok(module)
}

/// Returns the contained `Some` value or computes it from a closure.
///
/// # Examples
///
/// ```rune
/// let k = 10;
/// assert_eq!(Some(4).unwrap_or_else(|| 2 * k), 4);
/// assert_eq!(None.unwrap_or_else(|| 2 * k), 20);
/// ```
#[rune::function(instance)]
fn unwrap_or_else(this: &Option<Value>, default: Function) -> VmResult<Value> {
    VmResult::Ok(if let Some(this) = this {
        this.clone()
    } else {
        vm_try!(default.call(()))
    })
}

/// Transpose functions, translates an Option<Result<T, E>> into a `Result<Option<T>, E>`.
fn transpose_impl(this: &Option<Value>) -> VmResult<Value> {
    let value = match this {
        Some(value) => value,
        None => {
            let none = Value::from(Shared::new(Option::<Value>::None));
            let result = Value::from(Shared::new(Result::<Value, Value>::Ok(none)));
            return VmResult::Ok(result);
        }
    };

    let result = vm_try!(value.as_result());
    let result = vm_try!(result.borrow_ref());

    match &*result {
        Ok(ok) => {
            let some = Value::from(Shared::new(Option::<Value>::Some(ok.clone())));
            let result = Value::from(Shared::new(Result::<Value, Value>::Ok(some)));
            VmResult::Ok(result)
        }
        Err(err) => {
            let result = Value::from(Shared::new(Result::<Value, Value>::Err(err.clone())));
            VmResult::Ok(result)
        }
    }
}

fn option_iter(option: &Option<Value>) -> Iterator {
    Iterator::from_double_ended("std::option::Iter", option.clone().into_iter())
}

fn unwrap_impl(option: Option<Value>) -> VmResult<Value> {
    match option {
        Some(some) => VmResult::Ok(some),
        None => VmResult::err(Panic::custom("called `Option::unwrap()` on a `None` value")),
    }
}

fn expect_impl(option: Option<Value>, message: &str) -> VmResult<Value> {
    match option {
        Some(some) => VmResult::Ok(some),
        None => VmResult::err(Panic::custom(message.to_owned())),
    }
}

fn map_impl(option: &Option<Value>, then: Function) -> VmResult<Option<Value>> {
    match option {
        // no need to clone v, passing the same reference forward
        Some(v) => VmResult::Ok(Some(vm_try!(then.call::<_, _>((v,))))),
        None => VmResult::Ok(None),
    }
}

fn and_then_impl(option: &Option<Value>, then: Function) -> VmResult<Option<Value>> {
    match option {
        // no need to clone v, passing the same reference forward
        Some(v) => VmResult::Ok(vm_try!(then.call::<_, _>((v,)))),
        None => VmResult::Ok(None),
    }
}

fn take_impl(option: &mut Option<Value>) -> Option<Value> {
    option.take()
}
