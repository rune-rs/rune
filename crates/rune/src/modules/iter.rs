//! The `std::iter` module.

use crate::no_std::prelude::*;

use crate as rune;
use crate::runtime::{
    FromValue, Function, Iterator, Object, Protocol, Tuple, Value, Vec, VmResult,
};
use crate::{ContextError, Module};

/// Construct the `std::iter` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", ["iter"]);
    module.ty::<Iterator>()?;

    module.function_meta(next)?;
    module.function_meta(next_back)?;
    module.function_meta(find)?;
    module.function_meta(any)?;
    module.function_meta(all)?;
    module.function_meta(chain)?;
    module.function_meta(filter)?;
    module.function_meta(map)?;
    module.function_meta(flat_map)?;
    module.function_meta(enumerate)?;
    module.function_meta(peek)?;
    module.function_meta(peekable)?;
    module.function_meta(product)?;
    module.associated_function("fold", Iterator::fold)?;
    module.associated_function("rev", Iterator::rev)?;
    module.associated_function("size_hint", Iterator::size_hint)?;
    module.associated_function("sum", Iterator::sum)?;
    module.associated_function("skip", Iterator::skip)?;
    module.associated_function("take", Iterator::take)?;
    module.associated_function("count", Iterator::count)?;
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

/// Advances the iterator and returns the next value.
///
/// Returns [`None`] when iteration is finished. Individual iterator
/// implementations may choose to resume iteration, and so calling `next()`
/// again may or may not eventually start returning [`Some(Item)`] again at some
/// point.
///
/// [`Some(Item)`]: Some
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// let a = [1, 2, 3];
///
/// let iter = a.iter();
///
/// // A call to next() returns the next value...
/// assert_eq!(Some(1), iter.next());
/// assert_eq!(Some(2), iter.next());
/// assert_eq!(Some(3), iter.next());
///
/// // ... and then None once it's over.
/// assert_eq!(None, iter.next());
///
/// // More calls may or may not return `None`. Here, they always will.
/// assert_eq!(None, iter.next());
/// assert_eq!(None, iter.next());
/// ```
#[rune::function(instance)]
#[inline]
pub(crate) fn next(this: &mut Iterator) -> VmResult<Option<Value>> {
    this.next()
}

/// Removes and returns an element from the end of the iterator.
///
/// Returns `None` when there are no more elements.
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// let numbers = [1, 2, 3, 4, 5, 6];
///
/// let iter = numbers.iter();
///
/// assert_eq!(Some(1), iter.next());
/// assert_eq!(Some(6), iter.next_back());
/// assert_eq!(Some(5), iter.next_back());
/// assert_eq!(Some(2), iter.next());
/// assert_eq!(Some(3), iter.next());
/// assert_eq!(Some(4), iter.next());
/// assert_eq!(None, iter.next());
/// assert_eq!(None, iter.next_back());
/// ```
#[rune::function(instance)]
#[inline]
pub(crate) fn next_back(this: &mut Iterator) -> VmResult<Option<Value>> {
    this.next_back()
}

/// Searches for an element of an iterator that satisfies a predicate.
///
/// `find()` takes a closure that returns `true` or `false`. It applies this
/// closure to each element of the iterator, and if any of them return `true`,
/// then `find()` returns [`Some(element)`]. If they all return `false`, it
/// returns [`None`].
///
/// `find()` is short-circuiting; in other words, it will stop processing as
/// soon as the closure returns `true`.
///
/// If you need the index of the element, see [`position()`].
///
/// [`Some(element)`]: Some
/// [`position()`]: Iterator::position
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// let a = [1, 2, 3];
///
/// assert_eq!(a.iter().find(|x| x == 2), Some(2));
///
/// assert_eq!(a.iter().find(|x| x == 5), None);
/// ```
///
/// Stopping at the first `true`:
///
/// ```rune
/// let a = [1, 2, 3];
///
/// let iter = a.iter();
///
/// assert_eq!(iter.find(|x| x == 2), Some(2));
///
/// // we can still use `iter`, as there are more elements.
/// assert_eq!(iter.next(), Some(3));
/// ```
///
/// Note that `iter.find(f)` is equivalent to `iter.filter(f).next()`.
#[rune::function(instance)]
#[inline]
pub(crate) fn find(this: Iterator, find: Function) -> VmResult<Option<Value>> {
    this.find(find)
}

/// Tests if any element of the iterator matches a predicate.
///
/// `any()` takes a closure that returns `true` or `false`. It applies this
/// closure to each element of the iterator, and if any of them return `true`,
/// then so does `any()`. If they all return `false`, it returns `false`.
///
/// `any()` is short-circuiting; in other words, it will stop processing as soon
/// as it finds a `true`, given that no matter what else happens, the result
/// will also be `true`.
///
/// An empty iterator returns `false`.
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// let a = [1, 2, 3];
///
/// assert!(a.iter().any(|x| x > 0));
///
/// assert!(!a.iter().any(|x| x > 5));
/// ```
///
/// Stopping at the first `true`:
///
/// ```rune
/// let a = [1, 2, 3];
///
/// let iter = a.iter();
///
/// assert!(iter.any(|x| x != 2));
///
/// // we can still use `iter`, as there are more elements.
/// assert_eq!(iter.next(), Some(2));
/// ```
#[rune::function(instance)]
#[inline]
pub fn any(this: Iterator, find: Function) -> VmResult<bool> {
    this.any(find)
}

/// Tests if every element of the iterator matches a predicate.
///
/// `all()` takes a closure that returns `true` or `false`. It applies this
/// closure to each element of the iterator, and if they all return `true`, then
/// so does `all()`. If any of them return `false`, it returns `false`.
///
/// `all()` is short-circuiting; in other words, it will stop processing as soon
/// as it finds a `false`, given that no matter what else happens, the result
/// will also be `false`.
///
/// An empty iterator returns `true`.
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// let a = [1, 2, 3];
///
/// assert!(a.iter().all(|x| x > 0));
///
/// assert!(!a.iter().all(|x| x > 2));
/// ```
///
/// Stopping at the first `false`:
///
/// ```
/// let a = [1, 2, 3];
///
/// let iter = a.iter();
///
/// assert!(!iter.all(|x| x != 2));
///
/// // we can still use `iter`, as there are more elements.
/// assert_eq!(iter.next(), Some(3));
/// ```
#[rune::function(instance)]
#[inline]
pub fn all(this: Iterator, find: Function) -> VmResult<bool> {
    this.all(find)
}

/// Takes two iterators and creates a new iterator over both in sequence.
///
/// `chain()` will return a new iterator which will first iterate over
/// values from the first iterator and then over values from the second
/// iterator.
///
/// In other words, it links two iterators together, in a chain. 🔗
///
/// [`once`] is commonly used to adapt a single value into a chain of other
/// kinds of iteration.
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// let a1 = [1, 2, 3];
/// let a2 = [4, 5, 6];
///
/// let iter = a1.iter().chain(a2.iter());
///
/// assert_eq!(iter.next(), Some(1));
/// assert_eq!(iter.next(), Some(2));
/// assert_eq!(iter.next(), Some(3));
/// assert_eq!(iter.next(), Some(4));
/// assert_eq!(iter.next(), Some(5));
/// assert_eq!(iter.next(), Some(6));
/// assert_eq!(iter.next(), None);
/// ```
///
/// Since the argument to `chain()` uses [`INTO_ITER`], we can pass anything
/// that can be converted into an [`Iterator`], not just an [`Iterator`] itself.
/// For example, slices (`[T]`) implement [`INTO_ITER`], and so can be passed to
/// `chain()` directly:
///
/// ```rune
/// let s1 = [1, 2, 3];
/// let s2 = [4, 5, 6];
///
/// let iter = s1.iter().chain(s2);
///
/// assert_eq!(iter.next(), Some(1));
/// assert_eq!(iter.next(), Some(2));
/// assert_eq!(iter.next(), Some(3));
/// assert_eq!(iter.next(), Some(4));
/// assert_eq!(iter.next(), Some(5));
/// assert_eq!(iter.next(), Some(6));
/// assert_eq!(iter.next(), None);
/// ```
///
/// [`INTO_ITER`]: protocol@INTO_ITER
#[rune::function(instance)]
#[inline]
pub fn chain(this: Iterator, other: Value) -> VmResult<Iterator> {
    this.chain(other)
}

/// Creates an iterator which uses a closure to determine if an element
/// should be yielded.
///
/// Given an element the closure must return `true` or `false`. The returned
/// iterator will yield only the elements for which the closure returns
/// `true`.
///
/// ```rune
/// let a = [0, 1, 2];
///
/// let iter = a.iter().filter(|x| x.is_positive());
///
/// assert_eq!(iter.next(), Some(1));
/// assert_eq!(iter.next(), Some(2));
/// assert_eq!(iter.next(), None);
/// ```
#[rune::function(instance)]
#[inline]
fn filter(this: Iterator, filter: Function) -> Iterator {
    this.filter(filter)
}

/// Takes a closure and creates an iterator which calls that closure on each
/// element.
///
/// `map()` transforms one iterator into another. It produces a new iterator
/// which calls this closure on each element of the original iterator.
///
/// If you are good at thinking in types, you can think of `map()` like
/// this: If you have an iterator that gives you elements of some type `A`,
/// and you want an iterator of some other type `B`, you can use `map()`,
/// passing a closure that takes an `A` and returns a `B`.
///
/// `map()` is conceptually similar to a `for` loop. However, as `map()` is
/// lazy, it is best used when you're already working with other iterators.
/// If you're doing some sort of looping for a side effect, it's considered
/// more idiomatic to use `for` than `map()`.
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// let a = [1, 2, 3];
///
/// let iter = a.iter().map(|x| 2 * x);
///
/// assert_eq!(iter.next(), Some(2));
/// assert_eq!(iter.next(), Some(4));
/// assert_eq!(iter.next(), Some(6));
/// assert_eq!(iter.next(), None);
/// ```
///
/// If you're doing some sort of side effect, prefer `for` to `map()`:
///
/// ```rune
/// // don't do this:
/// (0..5).map(|x| println!("{x}"));
///
/// // it won't even execute, as it is lazy. Rust will warn you about this.
///
/// // Instead, use for:
/// for x in 0..5 {
///     println!("{x}");
/// }
/// ```
#[rune::function(instance)]
#[inline]
fn map(this: Iterator, map: Function) -> Iterator {
    this.map(map)
}

/// Creates an iterator that works like map, but flattens nested structure.
///
/// The [`map`] adapter is very useful, but only when the closure argument
/// produces values. If it produces an iterator instead, there's an extra
/// layer of indirection. `flat_map()` will remove this extra layer on its
/// own.
///
/// You can think of `flat_map(f)` as the semantic equivalent of
/// [`map`]ping, and then [`flatten`]ing as in `map(f).flatten()`.
///
/// Another way of thinking about `flat_map()`: [`map`]'s closure returns
/// one item for each element, and `flat_map()`'s closure returns an
/// iterator for each element.
///
/// [`map`]: Iterator::map
/// [`flatten`]: Iterator::flatten
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// let words = ["alpha", "beta", "gamma"];
///
/// // chars() returns an iterator
/// let merged = words.iter().flat_map(|s| s.chars()).collect::<String>();
/// assert_eq!(merged, "alphabetagamma");
/// ```
#[rune::function(instance)]
#[inline]
fn flat_map(this: Iterator, map: Function) -> Iterator {
    this.flat_map(map)
}

/// Creates an iterator which gives the current iteration count as well as
/// the next value.
///
/// The iterator returned yields pairs `(i, val)`, where `i` is the current
/// index of iteration and `val` is the value returned by the iterator.
///
/// `enumerate()` keeps its count as a usize. If you want to count by a
/// different sized integer, the zip function provides similar
/// functionality.
///
/// # Examples
///
/// ```rune
/// let a = ['a', 'b', 'c'];
///
/// let iter = a.iter().enumerate();
///
/// assert_eq!(iter.next(), Some((0, 'a')));
/// assert_eq!(iter.next(), Some((1, 'b')));
/// assert_eq!(iter.next(), Some((2, 'c')));
/// assert_eq!(iter.next(), None);
/// ```
#[rune::function(instance)]
#[inline]
fn enumerate(this: Iterator) -> Iterator {
    this.enumerate()
}

/// Returns a reference to the `next()` value without advancing the iterator.
///
/// Like [`next`], if there is a value, it is wrapped in a `Some(T)`. But if the
/// iteration is over, `None` is returned.
///
/// [`next`]: Iterator::next
///
/// Because `peek()` returns a reference, and many iterators iterate over
/// references, there can be a possibly confusing situation where the return
/// value is a double reference. You can see this effect in the examples below.
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// let xs = [1, 2, 3];
///
/// let iter = xs.iter().peekable();
///
/// // peek() lets us see into the future
/// assert_eq!(iter.peek(), Some(1));
/// assert_eq!(iter.next(), Some(1));
///
/// assert_eq!(iter.next(), Some(2));
///
/// // The iterator does not advance even if we `peek` multiple times
/// assert_eq!(iter.peek(), Some(3));
/// assert_eq!(iter.peek(), Some(3));
///
/// assert_eq!(iter.next(), Some(3));
///
/// // After the iterator is finished, so is `peek()`
/// assert_eq!(iter.peek(), None);
/// assert_eq!(iter.next(), None);
/// ```
#[rune::function(instance)]
#[inline]
fn peek(this: &mut Iterator) -> VmResult<Option<Value>> {
    this.peek()
}

/// Creates an iterator which can use the [`peek`] method to look at the next
/// element of the iterator without consuming it. See their documentation for
/// more information.
///
/// Note that the underlying iterator is still advanced when [`peek`] are called
/// for the first time: In order to retrieve the next element, [`next`] is
/// called on the underlying iterator, hence any side effects (i.e. anything
/// other than fetching the next value) of the [`next`] method will occur.
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// let xs = [1, 2, 3];
///
/// let iter = xs.iter().peekable();
///
/// // peek() lets us see into the future
/// assert_eq!(iter.peek(), Some(1));
/// assert_eq!(iter.next(), Some(1));
///
/// assert_eq!(iter.next(), Some(2));
///
/// // we can peek() multiple times, the iterator won't advance
/// assert_eq!(iter.peek(), Some(3));
/// assert_eq!(iter.peek(), Some(3));
///
/// assert_eq!(iter.next(), Some(3));
///
/// // after the iterator is finished, so is peek()
/// assert_eq!(iter.peek(), None);
/// assert_eq!(iter.next(), None);
/// ```
///
/// [`peek`]: Peekable::peek
/// [`next`]: Iterator::next
#[rune::function(instance)]
#[inline]
fn peekable(this: Iterator) -> Iterator {
    this.peekable()
}

#[rune::function(instance)]
#[inline]
fn product(this: Iterator) -> VmResult<Value> {
    this.product()
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
        match value {
            Value::Char(c) => {
                string.push(c);
            }
            Value::String(s) => {
                let s = vm_try!(s.into_ref());
                string.push_str(s.as_str());
            }
            Value::StaticString(s) => {
                string.push_str(s.as_str());
            }
            value => {
                return VmResult::expected::<String>(vm_try!(value.type_info()));
            }
        }
    }

    VmResult::Ok(string)
}
