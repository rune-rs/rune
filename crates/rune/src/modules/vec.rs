//! The `std::vec` module.

use core::cmp;

use crate as rune;
use crate::modules::collections::VecDeque;
use crate::runtime::{FromValue, Function, Protocol, Value, Vec, VmResult};
use crate::{ContextError, Module};

/// Construct the `std::vec` module.
pub fn module() -> Result<Module, ContextError> {
    let mut m = Module::with_crate_item("std", ["vec"]);

    m.ty::<Vec>()?;

    m.function_meta(vec_new)?;
    m.function_meta(vec_with_capacity)?;
    m.function_meta(len)?;
    m.function_meta(is_empty)?;
    m.function_meta(capacity)?;
    m.function_meta(get)?;
    m.function_meta(sort_by)?;
    m.function_meta(clear)?;
    m.associated_function("clone", Vec::clone)?;
    m.associated_function("extend", Vec::extend)?;
    m.associated_function("iter", Vec::into_iterator)?;
    m.associated_function("pop", Vec::pop)?;
    m.associated_function("push", Vec::push)?;
    m.associated_function("remove", Vec::remove)?;
    m.associated_function("insert", Vec::insert)?;
    m.associated_function(Protocol::INTO_ITER, Vec::into_iterator)?;
    m.associated_function(Protocol::INDEX_SET, Vec::set)?;
    m.associated_function(Protocol::EQ, eq)?;

    m.function_meta(sort_int)?;
    m.function_meta(into_vec_deque)?;
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
#[rune::function(path = Vec::new)]
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
#[rune::function(path = Vec::with_capacity)]
fn vec_with_capacity(capacity: usize) -> Vec {
    Vec::with_capacity(capacity)
}

/// Returns the number of elements in the vector, also referred to as its
/// 'length'.
///
/// # Examples
///
/// ```
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

/// Sort a vector of integers.
#[rune::function(instance, path = sort::<i64>)]
fn sort_int(vec: &mut Vec) {
    vec.sort_by(|a, b| match (a, b) {
        (Value::Integer(a), Value::Integer(b)) => a.cmp(b),
        // NB: fall back to sorting by address.
        _ => (a as *const _ as usize).cmp(&(b as *const _ as usize)),
    });
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
#[rune::function(instance, path = Vec::get)]
fn get(this: &Vec, index: Value) -> VmResult<Option<Value>> {
    let slice = match index {
        Value::RangeFrom(range) => {
            let range = vm_try!(range.borrow_ref());
            let start = vm_try!(range.start.as_usize());
            this.get(start..)
        }
        Value::RangeFull(..) => this.get(..),
        Value::RangeInclusive(range) => {
            let range = vm_try!(range.borrow_ref());
            let start = vm_try!(range.start.as_usize());
            let end = vm_try!(range.end.as_usize());
            this.get(start..=end)
        }
        Value::RangeToInclusive(range) => {
            let range = vm_try!(range.borrow_ref());
            let end = vm_try!(range.end.as_usize());
            this.get(..=end)
        }
        Value::RangeTo(range) => {
            let range = vm_try!(range.borrow_ref());
            let end = vm_try!(range.end.as_usize());
            this.get(..end)
        }
        Value::Range(range) => {
            let range = vm_try!(range.borrow_ref());
            let start = vm_try!(range.start.as_usize());
            let end = vm_try!(range.end.as_usize());
            this.get(start..end)
        }
        value => {
            let index = vm_try!(usize::from_value(value));

            let Some(value) = this.get(index) else {
                return VmResult::Ok(None);
            };

            return VmResult::Ok(Some(value.clone()));
        }
    };

    let Some(values) = slice else {
        return VmResult::Ok(None);
    };

    VmResult::Ok(Some(Value::vec(values.to_vec())))
}

/// Sort a vector by the specified comparator function.
///
/// # Examples
///
/// ```rune
/// let values = [1, 2, 3];
/// values.sort_by(|a, b| b.cmp(a))
/// ```
#[rune::function(instance)]
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
