//! The `std::object` module.

use crate::no_std::prelude::*;

use crate as rune;
use crate::runtime::{Iterator, Object, Protocol, Value};
use crate::{ContextError, Module};

/// Construct the `std::object` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", ["object"]);

    module.ty::<Object>()?;

    module.function_meta(Object::new__meta)?;
    module.function_meta(Object::with_capacity__meta)?;
    module.function_meta(Object::len__meta)?;
    module.function_meta(Object::is_empty__meta)?;
    module.function_meta(Object::insert__meta)?;
    module.function_meta(remove)?;
    module.function_meta(Object::clear__meta)?;
    module.function_meta(contains_key)?;
    module.function_meta(get)?;

    module.function_meta(Object::rune_iter__meta)?;
    module.associated_function(Protocol::INTO_ITER, Object::rune_iter)?;
    module.function_meta(keys)?;
    module.function_meta(values)?;
    Ok(module)
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
/// vec.sort::<int>();
/// assert_eq!(vec, ["a", "b", "c"]);
/// ```
#[rune::function(instance)]
#[inline]
fn keys(object: &Object) -> Iterator {
    let iter = object.keys().cloned().collect::<Vec<_>>().into_iter();
    Iterator::from_double_ended("std::object::Keys", iter)
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
/// vec.sort::<int>();
/// assert_eq!(vec, [1, 2, 3]);
/// ```
#[rune::function(instance)]
#[inline]
fn values(object: &Object) -> Iterator {
    let iter = object.values().cloned().collect::<Vec<_>>().into_iter();
    Iterator::from_double_ended("std::object::Values", iter)
}
