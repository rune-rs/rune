//! The `std::object` module.

use core::cmp::Ordering;

use crate::no_std::prelude::*;

use crate as rune;
use crate::runtime::{
    EnvProtocolCaller, FromValue, Iterator, Object, Protocol, Ref, Value, VmResult,
};
use crate::{ContextError, Module};

/// Construct the `std::object` module.
pub fn module() -> Result<Module, ContextError> {
    let mut m = Module::with_crate_item("std", ["object"]);

    m.ty::<Object>()?;

    m.function_meta(Object::new__meta)?;
    m.function_meta(Object::with_capacity__meta)?;
    m.function_meta(Object::len__meta)?;
    m.function_meta(Object::is_empty__meta)?;
    m.function_meta(Object::insert__meta)?;
    m.function_meta(remove)?;
    m.function_meta(Object::clear__meta)?;
    m.function_meta(contains_key)?;
    m.function_meta(get)?;

    m.function_meta(Object::rune_iter__meta)?;
    m.function_meta(keys)?;
    m.function_meta(values)?;
    m.associated_function(Protocol::INTO_ITER, Object::rune_iter)?;
    m.associated_function(Protocol::PARTIAL_EQ, partial_eq)?;
    m.associated_function(Protocol::EQ, eq)?;
    m.associated_function(Protocol::PARTIAL_CMP, partial_cmp)?;
    m.associated_function(Protocol::CMP, cmp)?;
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
/// vec.sort::<String>();
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
/// vec.sort::<i64>();
/// assert_eq!(vec, [1, 2, 3]);
/// ```
#[rune::function(instance)]
#[inline]
fn values(object: &Object) -> Iterator {
    let iter = object.values().cloned().collect::<Vec<_>>().into_iter();
    Iterator::from_double_ended("std::object::Values", iter)
}

fn partial_eq(this: &Object, other: Value) -> VmResult<bool> {
    let mut other = vm_try!(other.into_iter());

    for (k1, v1) in this.iter() {
        let Some(value) = vm_try!(other.next()) else {
            return VmResult::Ok(false);
        };

        let (k2, v2) = vm_try!(<(Ref<String>, Value)>::from_value(value));

        if k1 != &*k2 {
            return VmResult::Ok(false);
        }

        if !vm_try!(Value::partial_eq(v1, &v2)) {
            return VmResult::Ok(false);
        }
    }

    if vm_try!(other.next()).is_some() {
        return VmResult::Ok(false);
    }

    VmResult::Ok(true)
}

fn eq(this: &Object, other: &Object) -> VmResult<bool> {
    Object::eq_with(this, other, &mut EnvProtocolCaller)
}

fn partial_cmp(this: &Object, other: &Object) -> VmResult<Option<Ordering>> {
    Object::partial_cmp_with(this, other, &mut EnvProtocolCaller)
}

fn cmp(this: &Object, other: &Object) -> VmResult<Ordering> {
    Object::cmp_with(this, other, &mut EnvProtocolCaller)
}
