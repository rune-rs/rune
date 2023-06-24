//! The `std::iter` module.

use crate::no_std::prelude::*;

use crate as rune;
use crate::runtime::{FromValue, Iterator, Object, Protocol, Tuple, Value, Vec, VmResult};
use crate::{ContextError, Module};

/// Construct the `std::iter` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", ["iter"]);
    module.ty::<Iterator>()?;

    // Sorted for ease of finding
    module.function_meta(Iterator::chain__meta)?;
    module.function_meta(Iterator::enumerate__meta)?;
    module.function_meta(Iterator::filter__meta)?;
    module.function_meta(Iterator::find__meta)?;
    module.function_meta(Iterator::map__meta)?;
    module.function_meta(Iterator::flat_map__meta)?;
    module.associated_function("next", Iterator::next)?;
    module.associated_function("next_back", Iterator::next_back)?;
    module.associated_function("peek", Iterator::peek)?;
    module.associated_function("peekable", Iterator::peekable)?;
    module.associated_function("product", Iterator::product)?;
    module.associated_function("fold", Iterator::fold)?;
    module.associated_function("rev", Iterator::rev)?;
    module.associated_function("size_hint", Iterator::size_hint)?;
    module.associated_function("sum", Iterator::sum)?;
    module.associated_function("skip", Iterator::skip)?;
    module.associated_function("take", Iterator::take)?;
    module.associated_function("count", Iterator::count)?;
    module.associated_function("all", Iterator::all)?;
    module.associated_function(Protocol::NEXT, Iterator::next)?;
    module.associated_function(Protocol::INTO_ITER, <Iterator as From<Iterator>>::from)?;

    module.function_meta(range)?;
    module.function_meta(empty)?;
    module.function_meta(once)?;

    module.function_meta(collect_vec)?;
    module.function_meta(collect_tuple)?;
    module.function_meta(collect_object)?;
    module.function_meta(collect_string)?;
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

/// Collect the iterator as a [`Vec`].
///
/// # Examples
///
/// ```rune
/// use std::iter::range;
///
/// assert_eq!(range(0, 3).collect::<Vec>(), [0, 1, 2]);
/// ```
#[rune::function(instance, path = collect::<Vec>)]
fn collect_vec(it: Iterator) -> VmResult<Vec> {
    VmResult::Ok(Vec::from(vm_try!(it.collect::<Value>())))
}

/// Collect the iterator as a [`Tuple`].
///
/// # Examples
///
/// ```rune
/// use std::iter::range;
///
/// assert_eq!(range(0, 3).collect::<Tuple>(), (0, 1, 2));
/// ```
#[rune::function(instance, path = collect::<Tuple>)]
fn collect_tuple(it: Iterator) -> VmResult<Tuple> {
    VmResult::Ok(Tuple::from(vm_try!(it.collect::<Value>())))
}

/// Collect the iterator as an [`Object`].
///
/// # Examples
///
/// ```rune
/// assert_eq!([("first", 1), ("second", 2)].iter().collect::<Object>(), #{first: 1, second: 2});
/// ```
#[rune::function(instance, path = collect::<Object>)]
fn collect_object(mut it: Iterator) -> VmResult<Object> {
    let (cap, _) = it.size_hint();
    let mut object = Object::with_capacity(cap);

    while let Some(value) = vm_try!(it.next()) {
        let (key, value) = vm_try!(<(String, Value)>::from_value(value));
        object.insert(key, value);
    }

    VmResult::Ok(object)
}

/// Collect the iterator as a [`String`].
///
/// # Examples
///
/// ```rune
/// assert_eq!(["first", "second"].iter().collect::<String>(), "firstsecond");
/// ```
#[rune::function(instance, path = collect::<String>)]
fn collect_string(mut it: Iterator) -> VmResult<String> {
    let mut string = String::new();

    while let Some(value) = vm_try!(it.next()) {
        let s = vm_try!(String::from_value(value));
        string.push_str(s.as_str());
    }

    VmResult::Ok(string)
}
