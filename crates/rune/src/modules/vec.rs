//! The `std::vec` module.

use core::cmp;

use crate as rune;
use crate::modules::collections::VecDeque;
use crate::runtime::{Function, Protocol, Value, Vec, VmResult};
use crate::{ContextError, Module};

/// Construct the `std::vec` module.
pub fn module() -> Result<Module, ContextError> {
    let mut m = Module::with_crate_item("std", ["vec"]);

    m.ty::<Vec>()?;

    m.function(["Vec", "new"], Vec::new)?;
    m.associated_function("clear", Vec::clear)?;
    m.associated_function("clone", Vec::clone)?;
    m.associated_function("extend", Vec::extend)?;
    m.function_meta(get)?;
    m.associated_function("iter", Vec::into_iterator)?;
    m.associated_function("len", Vec::len)?;
    m.associated_function("pop", Vec::pop)?;
    m.associated_function("push", Vec::push)?;
    m.associated_function("remove", Vec::remove)?;
    m.function_meta(sort_by)?;
    m.associated_function("insert", Vec::insert)?;
    m.associated_function(Protocol::INTO_ITER, Vec::into_iterator)?;
    m.associated_function(Protocol::INDEX_SET, Vec::set)?;
    m.associated_function(Protocol::EQ, eq)?;

    m.function_meta(sort_int)?;
    m.function_meta(into_vec_deque)?;
    Ok(m)
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

/// Convert a vector into a vecdeque.
///
/// # Examples
///
/// ```rune
/// use std::collections::VecDeque;
///
/// let deque = [1, 2, 3].into::<VecDeque>();
///
/// assert_eq!(Some(1), deque.pop_front());
/// assert_eq!(Some(3), deque.pop_back());
/// ```
#[rune::function(instance, path = into::<VecDeque>)]
fn into_vec_deque(vec: Vec) -> VecDeque {
    VecDeque::from_vec(vec.into_inner())
}

fn eq(this: &Vec, other: Value) -> VmResult<bool> {
    let mut other = vm_try!(other.into_iter());

    for a in this.as_slice() {
        let Some(b) = vm_try!(other.next()) else {
            return VmResult::Ok(false);
        };

        if !vm_try!(Value::eq(a, &b)) {
            return VmResult::Ok(false);
        }
    }

    if vm_try!(other.next()).is_some() {
        return VmResult::Ok(false);
    }

    VmResult::Ok(true)
}
