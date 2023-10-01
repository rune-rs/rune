//! The `std::vec` module.

use core::cmp::Ordering;

use crate as rune;
use crate::alloc::prelude::*;
use crate::runtime::{
    EnvProtocolCaller, Formatter, Function, Hasher, Iterator, Ref, TypeOf, Value, Vec, VmErrorKind,
    VmResult,
};
use crate::{ContextError, Module};

/// Construct the `std::vec` module.
pub fn module() -> Result<Module, ContextError> {
    let mut m = Module::with_crate_item("std", ["vec"])?;

    m.ty::<Vec>()?.docs([
        "A dynamic vector.",
        "",
        "This is the type that is constructed in rune when an array expression such as `[1, 2, 3]` is used.",
        "",
        "# Comparisons",
        "",
        "Shorter sequences are considered smaller than longer ones, and vice versa.",
        "",
        "```rune",
        "assert!([1, 2, 3] < [1, 2, 3, 4]);",
        "assert!([1, 2, 3] < [1, 2, 4]);",
        "assert!([1, 2, 4] > [1, 2, 3]);",
        "```",
    ])?;

    m.function_meta(vec_new)?;
    m.function_meta(vec_with_capacity)?;
    m.function_meta(len)?;
    m.function_meta(is_empty)?;
    m.function_meta(capacity)?;
    m.function_meta(get)?;
    m.function_meta(clear)?;
    m.function_meta(extend)?;
    m.function_meta(iter)?;
    m.function_meta(pop)?;
    m.function_meta(push)?;
    m.function_meta(remove)?;
    m.function_meta(insert)?;
    m.function_meta(clone)?;
    m.function_meta(sort_by)?;
    m.function_meta(sort)?;
    m.function_meta(into_iter)?;
    m.function_meta(index_set)?;
    m.function_meta(index_get)?;
    m.function_meta(string_debug)?;
    m.function_meta(partial_eq)?;
    m.function_meta(eq)?;
    m.function_meta(partial_cmp)?;
    m.function_meta(cmp)?;
    m.function_meta(hash)?;
    Ok(m)
}

/// Constructs a new, empty dynamic `Vec`.
///
/// The vector will not allocate until elements are pushed onto it.
///
/// # Examples
///
/// ```rune
/// let vec = Vec::new();
/// ```
#[rune::function(free, path = Vec::new)]
fn vec_new() -> Vec {
    Vec::new()
}

/// Constructs a new, empty dynamic `Vec` with at least the specified capacity.
///
/// The vector will be able to hold at least `capacity` elements without
/// reallocating. This method is allowed to allocate for more elements than
/// `capacity`. If `capacity` is 0, the vector will not allocate.
///
/// It is important to note that although the returned vector has the minimum
/// *capacity* specified, the vector will have a zero *length*. For an
/// explanation of the difference between length and capacity, see *[Capacity
/// and reallocation]*.
///
/// If it is important to know the exact allocated capacity of a `Vec`, always
/// use the [`capacity`] method after construction.
///
/// [Capacity and reallocation]: #capacity-and-reallocation
/// [`capacity`]: Vec::capacity
///
/// # Panics
///
/// Panics if the new capacity exceeds `isize::MAX` bytes.
///
/// # Examples
///
/// ```rune
/// let vec = Vec::with_capacity(10);
///
/// // The vector contains no items, even though it has capacity for more
/// assert_eq!(vec.len(), 0);
/// assert!(vec.capacity() >= 10);
///
/// // These are all done without reallocating...
/// for i in 0..10 {
///     vec.push(i);
/// }
///
/// assert_eq!(vec.len(), 10);
/// assert!(vec.capacity() >= 10);
///
/// // ...but this may make the vector reallocate
/// vec.push(11);
/// assert_eq!(vec.len(), 11);
/// assert!(vec.capacity() >= 11);
/// ```
#[rune::function(free, path = Vec::with_capacity)]
fn vec_with_capacity(capacity: usize) -> VmResult<Vec> {
    VmResult::Ok(vm_try!(Vec::with_capacity(capacity)))
}

/// Returns the number of elements in the vector, also referred to as its
/// 'length'.
///
/// # Examples
///
/// ```rune
/// let a = [1, 2, 3];
/// assert_eq!(a.len(), 3);
/// ```
#[rune::function(instance)]
fn len(vec: &Vec) -> usize {
    vec.len()
}

/// Returns `true` if the vector contains no elements.
///
/// # Examples
///
/// ```rune
/// let v = Vec::new();
/// assert!(v.is_empty());
///
/// v.push(1);
/// assert!(!v.is_empty());
/// ```
#[rune::function(instance)]
fn is_empty(vec: &Vec) -> bool {
    vec.is_empty()
}

/// Returns the total number of elements the vector can hold without
/// reallocating.
///
/// # Examples
///
/// ```rune
/// let vec = Vec::with_capacity(10);
/// vec.push(42);
/// assert!(vec.capacity() >= 10);
/// ```
#[rune::function(instance)]
fn capacity(vec: &Vec) -> usize {
    vec.capacity()
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
/// let v = [1, 4, 3];
/// assert_eq!(Some(4), v.get(1));
/// assert_eq!(Some([1, 4]), v.get(0..2));
/// assert_eq!(Some([1, 4, 3]), v.get(0..=2));
/// assert_eq!(Some([1, 4, 3]), v.get(0..));
/// assert_eq!(Some([1, 4, 3]), v.get(..));
/// assert_eq!(Some([4, 3]), v.get(1..));
/// assert_eq!(None, v.get(3));
/// assert_eq!(None, v.get(0..4));
/// ```
#[rune::function(instance)]
fn get(this: &Vec, index: Value) -> VmResult<Option<Value>> {
    Vec::index_get(this, index)
}

/// Sort a vector by the specified comparator function.
///
/// # Examples
///
/// ```rune
/// use std::ops::cmp;
///
/// let values = [1, 2, 3];
/// values.sort_by(|a, b| cmp(b, a))
/// ```
#[rune::function(instance)]
fn sort_by(vec: &mut Vec, comparator: &Function) -> VmResult<()> {
    let mut error = None;

    vec.sort_by(|a, b| match comparator.call::<_, Ordering>((a, b)) {
        VmResult::Ok(ordering) => ordering,
        VmResult::Err(e) => {
            if error.is_none() {
                error = Some(e);
            }

            Ordering::Equal
        }
    });

    if let Some(e) = error {
        VmResult::Err(e)
    } else {
        VmResult::Ok(())
    }
}

/// Sort the vector.
///
/// This require all elements to be of the same type, and implement total
/// ordering per the [`CMP`] protocol.
///
/// # Panics
///
/// If any elements present are not comparable, this method will panic.
///
/// This will panic because a tuple and a string are not comparable:
///
/// ```rune,should_panic
/// let values = [(3, 1), "hello"];
/// values.sort();
/// ```
///
/// This too will panic because floating point values which do not have a total
/// ordering:
///
/// ```rune,should_panic
/// let values = [1.0, 2.0, f64::NAN];
/// values.sort();
/// ```
///
/// # Examples
///
/// ```rune
/// let values = [3, 2, 1];
/// values.sort();
/// assert_eq!(values, [1, 2, 3]);
///
/// let values = [(3, 1), (2, 1), (1, 1)];
/// values.sort();
/// assert_eq!(values, [(1, 1), (2, 1), (3, 1)]);
/// ```
#[rune::function(instance)]
fn sort(vec: &mut Vec) -> VmResult<()> {
    let mut err = None;

    vec.sort_by(|a, b| {
        let result: VmResult<Ordering> = Value::cmp(a, b);

        match result {
            VmResult::Ok(cmp) => cmp,
            VmResult::Err(e) => {
                if err.is_none() {
                    err = Some(e);
                }

                // NB: fall back to sorting by address.
                (a as *const _ as usize).cmp(&(b as *const _ as usize))
            }
        }
    });

    if let Some(err) = err {
        return VmResult::Err(err);
    }

    VmResult::Ok(())
}

/// Clears the vector, removing all values.
///
/// Note that this method has no effect on the allocated capacity of the vector.
///
/// # Examples
///
/// ```rune
/// let v = [1, 2, 3];
///
/// v.clear();
///
/// assert!(v.is_empty());
/// ```
#[rune::function(instance)]
fn clear(vec: &mut Vec) {
    vec.clear();
}

/// Extend these bytes with another collection.
///
/// # Examples
///
/// ```rune
/// let vec = [1, 2, 3, 4];
/// vec.extend([5, 6, 7, 8]);
/// assert_eq!(vec, [1, 2, 3, 4, 5, 6, 7, 8]);
/// ```
#[rune::function(instance)]
fn extend(this: &mut Vec, value: Value) -> VmResult<()> {
    this.extend(value)
}

/// Iterate over the collection.
///
/// # Examples
///
/// ```rune
/// let vec = [1, 2, 3, 4];
/// let it = vec.iter();
///
/// assert_eq!(Some(1), it.next());
/// assert_eq!(Some(4), it.next_back());
/// ```
#[rune::function(instance)]
fn iter(this: Ref<Vec>) -> Iterator {
    Vec::iter_ref(Ref::map(this, |vec| &**vec))
}

/// Removes the last element from a vector and returns it, or [`None`] if it is
/// empty.
///
/// If you'd like to pop the first element, consider using
/// [`VecDeque::pop_front`] instead.
///
/// [`VecDeque::pop_front`]: crate::collections::VecDeque::pop_front
///
/// # Examples
///
/// ```rune
/// let vec = [1, 2, 3];
/// assert_eq!(vec.pop(), Some(3));
/// assert_eq!(vec, [1, 2]);
/// ```
#[rune::function(instance)]
fn pop(this: &mut Vec) -> Option<Value> {
    this.pop()
}

/// Appends an element to the back of a collection.
///
/// # Panics
///
/// Panics if the new capacity exceeds `isize::MAX` bytes.
///
/// # Examples
///
/// ```rune
/// let vec = [1, 2];
/// vec.push(3);
/// assert_eq!(vec, [1, 2, 3]);
/// ```
#[rune::function(instance)]
fn push(this: &mut Vec, value: Value) -> VmResult<()> {
    vm_try!(this.push(value));
    VmResult::Ok(())
}

/// Removes and returns the element at position `index` within the vector,
/// shifting all elements after it to the left.
///
/// Note: Because this shifts over the remaining elements, it has a worst-case
/// performance of *O*(*n*). If you don't need the order of elements to be
/// preserved, use [`swap_remove`] instead. If you'd like to remove elements
/// from the beginning of the `Vec`, consider using [`VecDeque::pop_front`]
/// instead.
///
/// [`swap_remove`]: Vec::swap_remove
/// [`VecDeque::pop_front`]: crate::collections::VecDeque::pop_front
///
/// # Panics
///
/// Panics if `index` is out of bounds.
///
/// ```rune,should_panic
/// let v = [1, 2, 3];
/// v.remove(3);
/// ```
///
/// # Examples
///
/// ```rune
/// let v = [1, 2, 3];
/// assert_eq!(v.remove(1), 2);
/// assert_eq!(v, [1, 3]);
/// ```
#[rune::function(instance)]
fn remove(this: &mut Vec, index: usize) -> VmResult<Value> {
    if index >= this.len() {
        return VmResult::err(VmErrorKind::OutOfRange {
            index: index.into(),
            length: this.len().into(),
        });
    }

    let value = this.remove(index);
    VmResult::Ok(value)
}

/// Inserts an element at position `index` within the vector, shifting all
/// elements after it to the right.
///
/// # Panics
///
/// Panics if `index > len`.
///
/// # Examples
///
/// ```rune
/// let vec = [1, 2, 3];
/// vec.insert(1, 4);
/// assert_eq!(vec, [1, 4, 2, 3]);
/// vec.insert(4, 5);
/// assert_eq!(vec, [1, 4, 2, 3, 5]);
/// ```
#[rune::function(instance)]
fn insert(this: &mut Vec, index: usize, value: Value) -> VmResult<()> {
    if index > this.len() {
        return VmResult::err(VmErrorKind::OutOfRange {
            index: index.into(),
            length: this.len().into(),
        });
    }

    vm_try!(this.insert(index, value));
    VmResult::Ok(())
}

/// Clone the vector.
///
/// # Examples
///
/// ```rune
/// let a = [1, 2, 3];
/// let b = a.clone();
///
/// b.push(4);
///
/// assert_eq!(a, [1, 2, 3]);
/// assert_eq!(b, [1, 2, 3, 4]);
/// ```
#[rune::function(instance)]
fn clone(this: &Vec) -> VmResult<Vec> {
    VmResult::Ok(vm_try!(this.try_clone()))
}

/// Construct an iterator over the tuple.
///
/// # Examples
///
/// ```rune
/// let vec = [1, 2, 3];
/// let out = [];
///
/// for v in vec {
///     out.push(v);
/// }
///
/// assert_eq!(out, [1, 2, 3]);
/// ```
#[rune::function(instance, protocol = INTO_ITER)]
fn into_iter(this: Ref<Vec>) -> Iterator {
    Vec::iter_ref(Ref::map(this, |vec| &**vec))
}

/// Returns a reference to an element or subslice depending on the type of
/// index.
///
/// - If given a position, returns a reference to the element at that position
///   or `None` if out of bounds.
/// - If given a range, returns the subslice corresponding to that range, or
///   `None` if out of bounds.
///
/// # Panics
///
/// Panics if the specified `index` is out of range.
///
/// ```rune,should_panic
/// let v = [10, 40, 30];
/// assert_eq!(None, v[1..4]);
/// ```
///
/// ```rune,should_panic
/// let v = [10, 40, 30];
/// assert_eq!(None, v[3]);
/// ```
///
/// # Examples
///
/// ```rune
/// let v = [10, 40, 30];
/// assert_eq!(40, v[1]);
/// assert_eq!([10, 40], v[0..2]);
/// ```
#[rune::function(instance, protocol = INDEX_GET)]
fn index_get(this: &Vec, index: Value) -> VmResult<Value> {
    let Some(value) = vm_try!(Vec::index_get(this, index)) else {
        return VmResult::err(VmErrorKind::MissingIndex {
            target: Vec::type_info(),
        });
    };

    VmResult::Ok(value)
}

/// Inserts a value into the vector.
///
/// # Examples
///
/// ```rune
/// let vec = [1, 2, 3];
/// vec[0] = "a";
/// assert_eq!(vec, ["a", 2, 3]);
/// ```
#[rune::function(instance, protocol = INDEX_SET)]
fn index_set(this: &mut Vec, index: usize, value: Value) -> VmResult<()> {
    Vec::set(this, index, value)
}

/// Write a debug representation to a string.
///
/// This calls the [`STRING_DEBUG`] protocol over all elements of the
/// collection.
///
/// # Examples
///
/// ```rune
/// let vec = [1, 2, 3];
/// assert_eq!(format!("{:?}", vec), "[1, 2, 3]");
/// ```
#[rune::function(instance, protocol = STRING_DEBUG)]
fn string_debug(this: &Vec, f: &mut Formatter) -> VmResult<()> {
    Vec::string_debug_with(this, f, &mut EnvProtocolCaller)
}

/// Perform a partial equality check with this vector.
///
/// This can take any argument which can be converted into an iterator using
/// [`INTO_ITER`].
///
/// # Examples
///
/// ```rune
/// let vec = [1, 2, 3];
///
/// assert!(vec == [1, 2, 3]);
/// assert!(vec == (1..=3));
/// assert!(vec != [2, 3, 4]);
/// ```
#[rune::function(instance, protocol = PARTIAL_EQ)]
fn partial_eq(this: &Vec, other: Value) -> VmResult<bool> {
    Vec::partial_eq_with(this, other, &mut EnvProtocolCaller)
}

/// Perform a total equality check with this vector.
///
/// # Examples
///
/// ```rune
/// use std::ops::eq;
///
/// let vec = [1, 2, 3];
///
/// assert!(eq(vec, [1, 2, 3]));
/// assert!(!eq(vec, [2, 3, 4]));
/// ```
#[rune::function(instance, protocol = EQ)]
fn eq(this: &Vec, other: &Vec) -> VmResult<bool> {
    Vec::eq_with(this, other, Value::eq_with, &mut EnvProtocolCaller)
}

/// Perform a partial comparison check with this vector.
///
/// # Examples
///
/// ```rune
/// let vec = [1, 2, 3];
///
/// assert!(vec > [0, 2, 3]);
/// assert!(vec < [2, 2, 3]);
/// ```
#[rune::function(instance, protocol = PARTIAL_CMP)]
fn partial_cmp(this: &Vec, other: &Vec) -> VmResult<Option<Ordering>> {
    Vec::partial_cmp_with(this, other, &mut EnvProtocolCaller)
}

/// Perform a total comparison check with this vector.
///
/// # Examples
///
/// ```rune
/// use std::cmp::Ordering;
/// use std::ops::cmp;
///
/// let vec = [1, 2, 3];
///
/// assert_eq!(cmp(vec, [0, 2, 3]), Ordering::Greater);
/// assert_eq!(cmp(vec, [2, 2, 3]), Ordering::Less);
/// ```
#[rune::function(instance, protocol = CMP)]
fn cmp(this: &Vec, other: &Vec) -> VmResult<Ordering> {
    Vec::cmp_with(this, other, &mut EnvProtocolCaller)
}

/// Calculate the hash of a vector.
///
/// # Examples
///
/// ```rune
/// use std::ops::hash;
///
/// assert_eq!(hash([0, 2, 3]), hash([0, 2, 3]));
/// ```
#[rune::function(instance, protocol = HASH)]
fn hash(this: &Vec, hasher: &mut Hasher) -> VmResult<()> {
    Vec::hash_with(this, hasher, &mut EnvProtocolCaller)
}
