//! The `std::vec` module.

use core::cmp::Ordering;

use crate as rune;
use crate::runtime::{
    EnvProtocolCaller, FromValue, Function, Iterator, Protocol, Ref, Value, Vec, VmErrorKind,
    VmResult,
};
use crate::{ContextError, Module};

/// Construct the `std::vec` module.
pub fn module() -> Result<Module, ContextError> {
    let mut m = Module::with_crate_item("std", ["vec"]);

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
    ]);

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
    m.associated_function(Protocol::INTO_ITER, Vec::iter_ref)?;
    m.associated_function(Protocol::INDEX_SET, Vec::set)?;
    m.associated_function(Protocol::PARTIAL_EQ, partial_eq)?;
    m.associated_function(Protocol::EQ, eq)?;
    m.associated_function(Protocol::PARTIAL_CMP, partial_cmp)?;
    m.associated_function(Protocol::CMP, cmp)?;
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
/// This will panic because a tuple and a string is not comparable:
///
/// ```rune,should_panic
/// let values = [(3, 1), "hello"];
/// values.sort();
/// ```
///
/// This too will panic because floating point values do not have a total
/// ordering:
///
/// ```rune,should_panic
/// let values = [1.0, 2.0];
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
    Vec::iter_ref(this)
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
fn push(this: &mut Vec, value: Value) {
    this.push(value);
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

    this.insert(index, value);
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
fn clone(this: &Vec) -> Vec {
    this.clone()
}

fn partial_eq(this: &Vec, other: Value) -> VmResult<bool> {
    let mut other = vm_try!(other.into_iter());

    for a in this.as_slice() {
        let Some(b) = vm_try!(other.next()) else {
            return VmResult::Ok(false);
        };

        if !vm_try!(Value::partial_eq(a, &b)) {
            return VmResult::Ok(false);
        }
    }

    if vm_try!(other.next()).is_some() {
        return VmResult::Ok(false);
    }

    VmResult::Ok(true)
}

fn eq(this: &Vec, other: &Vec) -> VmResult<bool> {
    Vec::eq_with(this, other, &mut EnvProtocolCaller)
}

fn partial_cmp(this: &Vec, other: &Vec) -> VmResult<Option<Ordering>> {
    Vec::partial_cmp_with(this, other, &mut EnvProtocolCaller)
}

fn cmp(this: &Vec, other: &Vec) -> VmResult<Ordering> {
    Vec::cmp_with(this, other, &mut EnvProtocolCaller)
}
