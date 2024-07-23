//! The dynamic [`Object`] container.

use core::cmp::Ordering;

use crate as rune;
use crate::runtime::object::{RuneIter, RuneIterKeys, RuneValues};
use crate::runtime::{EnvProtocolCaller, Object, Protocol, Value, VmResult};
use crate::{ContextError, Module};

/// The dynamic [`Object`] container.
///
/// This modules contains the [`Object`] type, which is a dynamic type erased
/// container.
///
/// Objects in Rune are declared using the special `#{}` syntax, but can also be
/// interacted with through the fundamental [`Object`] type.
///
/// Fields can be added to objects "on the fly", simply by assigning to them:
///
/// ```rune
/// let object = #{};
/// object.hello = "World";
/// assert_eq!(object.hello, "World");
/// ```
///
/// # Examples
///
/// ```rune
/// let object1 = #{hello: "World"};
///
/// let object2 = Object::new();
/// object2.insert("hello", "World");
///
/// assert_eq!(object1, object2);
/// ```
#[rune::module(::std::object)]
pub fn module() -> Result<Module, ContextError> {
    let mut m = Module::from_meta(self::module_meta)?;

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
    m.function_meta(Object::rune_keys__meta)?;
    m.function_meta(Object::rune_values__meta)?;
    m.associated_function(Protocol::INTO_ITER, Object::rune_iter)?;
    m.function_meta(partial_eq)?;
    m.function_meta(eq)?;
    m.function_meta(partial_cmp)?;
    m.function_meta(cmp)?;

    m.ty::<RuneIter>()?;
    m.function_meta(RuneIter::next__meta)?;
    m.function_meta(RuneIter::into_iter__meta)?;

    m.ty::<RuneIterKeys>()?;
    m.function_meta(RuneIterKeys::next__meta)?;
    m.function_meta(RuneIterKeys::into_iter__meta)?;

    m.ty::<RuneValues>()?;
    m.function_meta(RuneValues::next__meta)?;
    m.function_meta(RuneValues::into_iter__meta)?;
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
