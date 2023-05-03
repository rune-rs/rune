//! The `std::vec` module.

use core::cmp;

use crate as rune;
use crate::runtime::{Function, Protocol, TypeOf, Value, Vec, VmResult};
use crate::{ContextError, Module, Params};

/// Construct the `std::vec` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", ["vec"]);

    module.ty::<Vec>()?;

    module.function(["Vec", "new"], Vec::new)?;
    module.inst_fn("clear", Vec::clear)?;
    module.inst_fn("clone", Vec::clone)?;
    module.inst_fn("extend", Vec::extend)?;
    module.function_meta(get)?;
    module.inst_fn("iter", Vec::into_iterator)?;
    module.inst_fn("len", Vec::len)?;
    module.inst_fn("pop", Vec::pop)?;
    module.inst_fn("push", Vec::push)?;
    module.inst_fn("remove", Vec::remove)?;
    module.function_meta(sort_by)?;
    module.inst_fn("insert", Vec::insert)?;
    module.inst_fn(Protocol::INTO_ITER, Vec::into_iterator)?;
    module.inst_fn(Protocol::INDEX_SET, Vec::set)?;

    // TODO: parameterize with generics.
    module.inst_fn(Params::new("sort", [i64::type_of()]), sort_int)?;
    Ok(module)
}

/// Sort a vector of integers.
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
