//! The [`Tuple`] fixed collection.

use core::cmp::Ordering;

use crate as rune;
use crate::runtime::slice::Iter;
use crate::runtime::{
    EnvProtocolCaller, Formatter, Hasher, OwnedTuple, Ref, Tuple, Value, Vec, VmError,
};
use crate::{docstring, ContextError, Module};

/// The [`Tuple`] fixed collection.
///
/// Tuples are anonymous types that can hold a fixed number of elements.
///
/// Tuples in Rune are declared with the special `(a)` syntax, but can also be
/// interacted with through the fundamental [`Tuple`] type.
///
/// Once a tuple has been declared, its size cannot change.
///
/// The tuple type has support for native pattern matching:
///
/// ```rune
/// let value = (1, 2);
///
/// if let (a, b) = value {
///     assert_eq!(a, 1);
///     assert_eq!(b, 2);
/// }
/// ```
///
/// # Examples
///
/// ```rune
/// let empty = ();
/// let one = (10,);
/// let two = (10, 20);
///
/// assert!(empty.is_empty());
/// assert_eq!(one.0, 10);
/// assert_eq!(two.0, 10);
/// assert_eq!(two.1, 20);
/// ```
#[rune::module(::std::tuple)]
pub fn module() -> Result<Module, ContextError> {
    let mut m = Module::from_meta(self::module__meta)?;
    m.ty::<OwnedTuple>()?.docs(docstring! {
        /// The tuple type.
    })?;
    m.function_meta(len)?;
    m.function_meta(is_empty)?;
    m.function_meta(get)?;
    m.function_meta(iter)?;
    m.function_meta(into_iter)?;

    m.function_meta(partial_eq__meta)?;
    m.implement_trait::<OwnedTuple>(rune::item!(::std::cmp::PartialEq))?;

    m.function_meta(eq__meta)?;
    m.implement_trait::<OwnedTuple>(rune::item!(::std::cmp::Eq))?;

    m.function_meta(partial_cmp__meta)?;
    m.implement_trait::<OwnedTuple>(rune::item!(::std::cmp::PartialOrd))?;

    m.function_meta(cmp__meta)?;
    m.implement_trait::<OwnedTuple>(rune::item!(::std::cmp::Ord))?;

    m.function_meta(clone__meta)?;
    m.implement_trait::<OwnedTuple>(rune::item!(::std::clone::Clone))?;

    m.function_meta(hash__meta)?;
    m.function_meta(debug_fmt__meta)?;
    Ok(m)
}

/// Returns the number of elements in the tuple.
///
/// # Examples
///
/// ```rune
/// let a = (1, 2, 3);
/// assert_eq!(a.len(), 3);
/// ```
#[rune::function(instance)]
fn len(this: &Tuple) -> usize {
    this.len()
}

/// Returns `true` if the tuple has a length of 0.
///
/// # Examples
///
/// ```rune
/// let a = (1, 2, 3);
/// assert!(!a.is_empty());
///
/// let a = ();
/// assert!(a.is_empty());
/// ```
#[rune::function(instance)]
fn is_empty(this: &Tuple) -> bool {
    this.is_empty()
}

/// Returns a reference to an element or subslice depending on the type of
/// index.
///
/// - If given a position, returns a reference to the element at that position
///   or `None` if out of bounds.
/// - If given a range, returns the subslice corresponding to that range, or
///   `None` if out of bounds.
///
/// # Examples
///
/// ```rune
/// let v = (10, 40, 30);
/// assert_eq!(Some(40), v.get(1));
/// assert_eq!(Some([10, 40]), v.get(0..2));
/// assert_eq!(None, v.get(3));
/// assert_eq!(None, v.get(0..4));
/// ```
#[rune::function(instance)]
fn get(this: &Tuple, index: Value) -> Result<Option<Value>, VmError> {
    Vec::index_get(this, index)
}

/// Construct an iterator over the tuple.
///
/// # Examples
///
/// ```rune
/// let tuple = (1, 2, 3);
/// assert_eq!(tuple.iter().collect::<Vec>(), [1, 2, 3]);
/// ```
#[rune::function(instance)]
fn iter(this: Ref<Tuple>) -> Iter {
    Iter::new(Ref::map(this, |tuple| &**tuple))
}

/// Construct an iterator over the tuple.
///
/// # Examples
///
/// ```rune
/// let tuple = (1, 2, 3);
/// let out = [];
///
/// for v in tuple {
///     out.push(v);
/// }
///
/// assert_eq!(out, [1, 2, 3]);
/// ```
#[rune::function(instance, instance, protocol = INTO_ITER)]
fn into_iter(this: Ref<Tuple>) -> Iter {
    Iter::new(Ref::map(this, |tuple| &**tuple))
}

/// Perform a partial equality check with this tuple.
///
/// This can take any argument which can be converted into an iterator using
/// [`INTO_ITER`].
///
/// # Examples
///
/// ```rune
/// let tuple = (1, 2, 3);
///
/// assert!(tuple == (1, 2, 3));
/// assert!(tuple == (1..=3));
/// assert!(tuple != (2, 3, 4));
/// ```
#[rune::function(keep, instance, protocol = PARTIAL_EQ)]
fn partial_eq(this: &Tuple, other: Value) -> Result<bool, VmError> {
    Vec::partial_eq_with(this, other, &mut EnvProtocolCaller)
}

/// Perform a total equality check with this tuple.
///
/// # Examples
///
/// ```rune
/// use std::ops::eq;
///
/// let tuple = (1, 2, 3);
///
/// assert!(eq(tuple, (1, 2, 3)));
/// assert!(!eq(tuple, (2, 3, 4)));
/// ```
#[rune::function(keep, instance, protocol = EQ)]
fn eq(this: &Tuple, other: &Tuple) -> Result<bool, VmError> {
    Vec::eq_with(this, other, Value::eq_with, &mut EnvProtocolCaller)
}

/// Perform a partial comparison check with this tuple.
///
/// # Examples
///
/// ```rune
/// let tuple = (1, 2, 3);
///
/// assert!(tuple > (0, 2, 3));
/// assert!(tuple < (2, 2, 3));
/// ```
#[rune::function(keep, instance, protocol = PARTIAL_CMP)]
fn partial_cmp(this: &Tuple, other: &Tuple) -> Result<Option<Ordering>, VmError> {
    Vec::partial_cmp_with(this, other, &mut EnvProtocolCaller)
}

/// Perform a total comparison check with this tuple.
///
/// # Examples
///
/// ```rune
/// use std::cmp::Ordering;
/// use std::ops::cmp;
///
/// let tuple = (1, 2, 3);
///
/// assert_eq!(cmp(tuple, (0, 2, 3)), Ordering::Greater);
/// assert_eq!(cmp(tuple, (2, 2, 3)), Ordering::Less);
/// ```
#[rune::function(keep, instance, protocol = CMP)]
fn cmp(this: &Tuple, other: &Tuple) -> Result<Ordering, VmError> {
    Vec::cmp_with(this, other, &mut EnvProtocolCaller)
}

/// Calculate a hash for a tuple.
///
/// # Examples
///
/// ```rune
/// use std::ops::hash;
///
/// assert_eq!(hash((0, 2, 3)), hash((0, 2, 3)));
/// // Note: this is not guaranteed to be true forever, but it's true right now.
/// assert_eq!(hash((0, 2, 3)), hash([0, 2, 3]));
/// ```
#[rune::function(keep, instance, protocol = HASH)]
fn hash(this: &Tuple, hasher: &mut Hasher) -> Result<(), VmError> {
    Tuple::hash_with(this, hasher, &mut EnvProtocolCaller)
}

/// Clone a tuple.
///
/// # Examples
///
/// ```rune
/// use std::ops::hash;
///
/// let a = (0, 2, 3);
/// let b = a;
/// let c = a.clone();
///
/// c.0 = 1;
///
/// assert_eq!(a, (0, 2, 3));
/// assert_eq!(c, (1, 2, 3));
/// ```
#[rune::function(keep, instance, protocol = CLONE)]
fn clone(this: &Tuple) -> Result<OwnedTuple, VmError> {
    Ok(this.clone_with(&mut EnvProtocolCaller)?)
}

/// Write a debug representation of a tuple.
///
/// # Examples
///
/// ```rune
/// let a = (1, 2, 3);
/// println!("{a:?}");
/// ```
#[rune::function(keep, instance, protocol = DEBUG_FMT)]
#[inline]
fn debug_fmt(this: &Tuple, f: &mut Formatter) -> Result<(), VmError> {
    this.debug_fmt_with(f, &mut EnvProtocolCaller)
}
