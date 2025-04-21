//! The dynamic [`Object`] container.

use crate as rune;
use crate::alloc::clone::TryClone;
use crate::alloc::fmt::TryWrite;
use crate::runtime::object::{RuneIter, RuneIterKeys, RuneValues};
use crate::runtime::{EnvProtocolCaller, Formatter, Object, Protocol, Value, VmResult};
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
    let mut m = Module::from_meta(self::module__meta)?;

    m.ty::<Object>()?;

    m.function_meta(Object::new__meta)?;
    m.function_meta(Object::rune_with_capacity)?;
    m.function_meta(Object::len__meta)?;
    m.function_meta(Object::is_empty__meta)?;
    m.function_meta(Object::rune_insert)?;
    m.function_meta(remove__meta)?;
    m.function_meta(Object::clear__meta)?;
    m.function_meta(contains_key__meta)?;
    m.function_meta(get__meta)?;

    m.function_meta(partial_eq__meta)?;
    m.implement_trait::<Object>(rune::item!(::std::cmp::PartialEq))?;

    m.function_meta(eq__meta)?;
    m.implement_trait::<Object>(rune::item!(::std::cmp::Eq))?;

    m.function_meta(clone__meta)?;
    m.implement_trait::<Object>(rune::item!(::std::clone::Clone))?;

    m.function_meta(debug_fmt__meta)?;

    m.function_meta(Object::rune_iter__meta)?;
    m.function_meta(Object::rune_keys__meta)?;
    m.function_meta(Object::rune_values__meta)?;
    m.associated_function(&Protocol::INTO_ITER, Object::rune_iter)?;

    m.ty::<RuneIter>()?;
    m.function_meta(RuneIter::next__meta)?;
    m.function_meta(RuneIter::size_hint__meta)?;
    m.implement_trait::<RuneIter>(rune::item!(::std::iter::Iterator))?;
    m.function_meta(RuneIter::len__meta)?;
    m.implement_trait::<RuneIter>(rune::item!(::std::iter::ExactSizeIterator))?;

    m.ty::<RuneIterKeys>()?;
    m.function_meta(RuneIterKeys::next__meta)?;
    m.function_meta(RuneIterKeys::size_hint__meta)?;
    m.implement_trait::<RuneIterKeys>(rune::item!(::std::iter::Iterator))?;
    m.function_meta(RuneIterKeys::len__meta)?;
    m.implement_trait::<RuneIterKeys>(rune::item!(::std::iter::ExactSizeIterator))?;

    m.ty::<RuneValues>()?;
    m.function_meta(RuneValues::next__meta)?;
    m.function_meta(RuneValues::size_hint__meta)?;
    m.implement_trait::<RuneValues>(rune::item!(::std::iter::Iterator))?;
    m.function_meta(RuneValues::len__meta)?;
    m.implement_trait::<RuneValues>(rune::item!(::std::iter::ExactSizeIterator))?;
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
#[rune::function(keep, instance)]
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
#[rune::function(keep, instance)]
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
#[rune::function(keep, instance)]
#[inline]
fn get(object: &Object, key: &str) -> Option<Value> {
    object.get(key).cloned()
}

/// Test two objects for partial equality.
///
/// # Examples
///
/// ```rune
/// let a = #{a: 42};
/// let b = #{a: 43};
///
/// assert_eq!(a, a);
/// assert_ne!(a, b);
/// assert_ne!(b, a);
/// ```
#[rune::function(keep, instance, protocol = PARTIAL_EQ)]
fn partial_eq(this: &Object, other: &Object) -> VmResult<bool> {
    Object::partial_eq_with(this, other, &mut EnvProtocolCaller)
}

/// Test two objects for total equality.
///
/// # Examples
///
/// ```rune
/// use std::ops::eq;
///
/// let a = #{a: 42};
/// let b = #{a: 43};
///
/// assert_eq!(eq(a, a), true);
/// assert_eq!(eq(a, b), false);
/// assert_eq!(eq(b, a), false);
/// ```
#[rune::function(keep, instance, protocol = EQ)]
fn eq(this: &Object, other: &Object) -> VmResult<bool> {
    Object::eq_with(this, other, Value::eq_with, &mut EnvProtocolCaller)
}

/// Clones an object.
///
/// # Examples
///
/// ```rune
/// let a = #{a: 42};
/// let b = a.clone();
/// assert_eq!(a, b);
///
/// b.b = 43;
/// assert_ne!(a, b);
/// ```
#[rune::function(keep, instance, protocol = CLONE)]
fn clone(this: &Object) -> VmResult<Object> {
    VmResult::Ok(vm_try!(this.try_clone()))
}

/// Write a debug representation of an object.
///
/// # Examples
///
/// ```rune
/// let a = #{a: 42, b: 43};
///
/// println!("{a:?}");
/// ```
#[rune::function(keep, instance, protocol = DEBUG_FMT)]
#[inline]
fn debug_fmt(this: &Object, f: &mut Formatter) -> VmResult<()> {
    vm_write!(f, "{this:?}")
}
