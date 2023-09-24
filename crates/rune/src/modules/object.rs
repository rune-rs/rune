//! The `std::object` module.

use core::cmp::Ordering;

use crate as rune;
use crate::alloc::prelude::*;
use crate::alloc::Vec;
use crate::runtime::{EnvProtocolCaller, Iterator, Object, Protocol, Value, VmResult};
use crate::{ContextError, Module};

/// Construct the `std::object` module.
pub fn module() -> Result<Module, ContextError> {
    let mut m = Module::with_crate_item("std", ["object"])?;

    m.ty::<Object>()?;

    m.function_meta(Object::new__meta)?;
    m.function_meta(Object::rune_with_capacity)?;
    m.function_meta(Object::len__meta)?;
    m.function_meta(Object::is_empty__meta)?;
    m.function_meta(Object::rune_insert)?;
    m.function_meta(remove)?;
    m.function_meta(Object::clear__meta)?;
    m.function_meta(contains_key)?;
    m.function_meta(get)?;

    m.function_meta(Object::rune_iter__meta)?;
    m.function_meta(keys)?;
    m.function_meta(values)?;
    m.associated_function(Protocol::INTO_ITER, Object::rune_iter)?;
    m.function_meta(partial_eq)?;
    m.function_meta(eq)?;
    m.function_meta(partial_cmp)?;
    m.function_meta(cmp)?;
    Ok(m)
}

/// Returns `true` if the map contains a value for the specified key.
///
/// # Examples
///
/// ```rune
/// let object = #{a: 42};
/// assert!(object.contains_key("a"));
/// ```
#[rune::function(instance)]
#[inline]
fn contains_key(object: &Object, key: &str) -> bool {
    object.contains_key(key)
}

/// Removes a key from the map, returning the value at the key if the key was
/// previously in the map.
///
/// # Examples
///
/// ```rune
/// let object = #{a: 42};
/// assert_eq!(object.remove("a"), Some(42));
/// assert_eq!(object.remove("a"), None);
/// ```
#[rune::function(instance)]
#[inline]
fn remove(object: &mut Object, key: &str) -> Option<Value> {
    object.remove(key)
}

/// Returns a reference to the value corresponding to the key.
///
/// # Examples
///
/// ```rune
/// let object = #{a: 42};
/// assert_eq!(object.get("a"), Some(42));
/// assert_eq!(object.get("b"), None);
/// ```
#[rune::function(instance)]
#[inline]
fn get(object: &Object, key: &str) -> Option<Value> {
    object.get(key).cloned()
}

/// An iterator visiting all keys in arbitrary order.
///
/// # Examples
///
/// ```rune
/// let object = #{a: 1, b: 2, c: 3};
/// let vec = [];
///
/// for key in object.keys() {
///     vec.push(key);
/// }
///
/// vec.sort();
/// assert_eq!(vec, ["a", "b", "c"]);
/// ```
#[inline]
#[rune::function(vm_result, instance)]
fn keys(object: &Object) -> Iterator {
    // TODO: implement as lazy iteration.
    let mut keys = Vec::new();

    for key in object.keys() {
        keys.try_push(key.try_clone().vm?).vm?;
    }

    Iterator::from_double_ended("std::object::Keys", keys.into_iter())
}

/// An iterator visiting all values in arbitrary order.
///
/// # Examples
///
/// ```rune
/// let object = #{a: 1, b: 2, c: 3};
/// let vec = [];
///
/// for key in object.values() {
///     vec.push(key);
/// }
///
/// vec.sort();
/// assert_eq!(vec, [1, 2, 3]);
/// ```
#[inline]
#[rune::function(vm_result, instance)]
fn values(object: &Object) -> Iterator {
    // TODO: implement as lazy iteration.
    let iter = object
        .values()
        .cloned()
        .try_collect::<Vec<_>>()
        .vm?
        .into_iter();
    Iterator::from_double_ended("std::object::Values", iter)
}

#[rune::function(instance, protocol = PARTIAL_EQ)]
fn partial_eq(this: &Object, other: Value) -> VmResult<bool> {
    Object::partial_eq_with(this, other, &mut EnvProtocolCaller)
}

#[rune::function(instance, protocol = EQ)]
fn eq(this: &Object, other: &Object) -> VmResult<bool> {
    Object::eq_with(this, other, Value::eq_with, &mut EnvProtocolCaller)
}

#[rune::function(instance, protocol = PARTIAL_CMP)]
fn partial_cmp(this: &Object, other: &Object) -> VmResult<Option<Ordering>> {
    Object::partial_cmp_with(this, other, &mut EnvProtocolCaller)
}

#[rune::function(instance, protocol = CMP)]
fn cmp(this: &Object, other: &Object) -> VmResult<Ordering> {
    Object::cmp_with(this, other, &mut EnvProtocolCaller)
}
