//! The `std::iter` module.

use crate as rune;
use crate::runtime::{FromValue, Iterator, Object, Protocol, Tuple, TypeOf, Value, Vec, VmResult};
use crate::{ContextError, Module, Params};

/// Construct the `std::iter` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", ["iter"]);
    module.ty::<Iterator>()?;

    // Sorted for ease of finding
    module.inst_fn("chain", Iterator::chain)?;
    module.inst_fn(Params::new("collect", [Object::type_of()]), collect_object)?;
    module.inst_fn(Params::new("collect", [Vec::type_of()]), collect_vec)?;
    module.inst_fn(Params::new("collect", [Tuple::type_of()]), collect_tuple)?;
    module.inst_fn("enumerate", Iterator::enumerate)?;
    module.inst_fn("filter", Iterator::filter)?;
    module.inst_fn("find", Iterator::find)?;
    module.inst_fn("flat_map", Iterator::flat_map)?;
    module.inst_fn("map", Iterator::map)?;
    module.inst_fn("next", Iterator::next)?;
    module.inst_fn("next_back", Iterator::next_back)?;
    module.inst_fn("peek", Iterator::peek)?;
    module.inst_fn("peekable", Iterator::peekable)?;
    module.inst_fn("product", Iterator::product)?;
    module.inst_fn("fold", Iterator::fold)?;
    module.inst_fn("rev", Iterator::rev)?;
    module.inst_fn("size_hint", Iterator::size_hint)?;
    module.inst_fn("sum", Iterator::sum)?;
    module.inst_fn("skip", Iterator::skip)?;
    module.inst_fn("take", Iterator::take)?;
    module.inst_fn("count", Iterator::count)?;
    module.inst_fn("all", Iterator::all)?;
    module.inst_fn(Protocol::NEXT, Iterator::next)?;
    module.inst_fn(Protocol::INTO_ITER, <Iterator as From<Iterator>>::from)?;

    module.function_meta(range)?;
    module.function_meta(empty)?;
    module.function_meta(once)?;
    Ok(module)
}

/// Construct an iterator which produces no values.
///
/// # Examples
///
/// ```rune
/// use std::iter::empty;
///
/// assert!(empty().next().is_none());
/// assert_eq!(empty().collect::<Vec>(), []);
/// ```
#[rune::function]
fn empty() -> Iterator {
    Iterator::empty()
}

/// Construct an iterator which produces a single `value` once.
///
/// # Examples
///
/// ```rune
/// use std::iter::once;
///
/// assert!(once(42).next().is_some());
/// assert_eq!(once(42).collect::<Vec>(), [42]);
/// ```
#[rune::function]
fn once(value: Value) -> Iterator {
    Iterator::once(value)
}

/// Produce an iterator which starts at the range `start` and ends at the value
/// `end` (exclusive).
///
/// # Examples
///
/// ```rune
/// use std::iter::range;
///
/// assert!(range(0, 3).next().is_some());
/// assert_eq!(range(0, 3).collect::<Vec>(), [0, 1, 2]);
/// ```
#[rune::function]
fn range(start: i64, end: i64) -> Iterator {
    Iterator::from_double_ended("std::iter::Range", start..end)
}

fn collect_vec(it: Iterator) -> VmResult<Vec> {
    VmResult::Ok(Vec::from(vm_try!(it.collect::<Value>())))
}

fn collect_tuple(it: Iterator) -> VmResult<Tuple> {
    VmResult::Ok(Tuple::from(vm_try!(it.collect::<Value>())))
}

fn collect_object(mut it: Iterator) -> VmResult<Object> {
    let (cap, _) = it.size_hint();
    let mut object = Object::with_capacity(cap);

    while let Some(value) = vm_try!(it.next()) {
        let (key, value) = vm_try!(<(String, Value)>::from_value(value));
        object.insert(key, value);
    }

    VmResult::Ok(object)
}
