//! The `std::vec` module.

use core::cmp;

use crate as rune;
use crate::runtime::{Function, Protocol, Value, Vec, VmResult};
use crate::{ContextError, Module};

/// Construct the `std::vec` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", ["vec"]);

    module.ty::<Vec>()?;

    module.function(["Vec", "new"], Vec::new)?;
    module.associated_function("clear", Vec::clear)?;
    module.associated_function("clone", Vec::clone)?;
    module.associated_function("extend", Vec::extend)?;
    module.function_meta(get)?;
    module.associated_function("iter", Vec::into_iterator)?;
    module.associated_function("len", Vec::len)?;
    module.associated_function("pop", Vec::pop)?;
    module.associated_function("push", Vec::push)?;
    module.associated_function("remove", Vec::remove)?;
    module.function_meta(sort_by)?;
    module.associated_function("insert", Vec::insert)?;
    module.associated_function(Protocol::INTO_ITER, Vec::into_iterator)?;
    module.associated_function(Protocol::INDEX_SET, Vec::set)?;

    module.function_meta(sort_int)?;
    Ok(module)
}

/// Sort a vector of integers.
#[rune::function(instance, path = sort::<i64>)]
fn sort_int(vec: &mut Vec) {
    vec.sort_by(|a, b| match (a, b) {
        (Value::Integer(a), Value::Integer(b)) => a.cmp(b),
        // NB: fall back to sorting by address.
        _ => (a as *const _ as usize).cmp(&(b as *const _ as usize)),
    });
}

/// Get a value by the specified `index`.
///
/// # Examples
///
/// ```rune
/// let values = [1, 2, 3];
/// assert!(values.get(1).is_some());
/// assert!(values.get(4).is_none());
/// ```
#[rune::function(instance, path = Vec::get)]
fn get(vec: &Vec, index: usize) -> Option<Value> {
    vec.get(index).cloned()
}

/// Sort a vector by the specified comparator function.
///
/// # Examples
///
/// ```rune
/// let values = [1, 2, 3];
/// values.sort_by(|a, b| b.cmp(a))
/// ```
#[rune::function(instance, path = Vec::sort_by)]
fn sort_by(vec: &mut Vec, comparator: &Function) -> VmResult<()> {
    let mut error = None;

    vec.sort_by(|a, b| match comparator.call::<_, cmp::Ordering>((a, b)) {
        VmResult::Ok(ordering) => ordering,
        VmResult::Err(e) => {
            if error.is_none() {
                error = Some(e);
            }

            cmp::Ordering::Equal
        }
    });

    if let Some(e) = error {
        VmResult::Err(e)
    } else {
        VmResult::Ok(())
    }
}
