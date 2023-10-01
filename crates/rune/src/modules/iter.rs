//! The `std::iter` module.

use crate as rune;
use crate::alloc::String;
use crate::modules::collections::VecDeque;
#[cfg(feature = "alloc")]
use crate::modules::collections::{HashMap, HashSet};
#[cfg(feature = "alloc")]
use crate::runtime::EnvProtocolCaller;
use crate::runtime::{
    FromValue, Function, Iterator, Object, OwnedTuple, Protocol, Value, Vec, VmResult,
};
use crate::{ContextError, Module};

/// Construct the `std::iter` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", ["iter"])?;
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
    module.function_meta(sum_int)?;
    module.function_meta(sum_byte)?;
    module.function_meta(sum_float)?;
    module.function_meta(product_int)?;
    module.function_meta(product_byte)?;
    module.function_meta(product_float)?;
    module.function_meta(fold)?;
    module.function_meta(reduce)?;
    module.function_meta(rev)?;
    module.function_meta(size_hint)?;
    module.function_meta(skip)?;
    module.function_meta(take)?;
    module.function_meta(count)?;
    module.associated_function(Protocol::NEXT, Iterator::next)?;
    module.associated_function(Protocol::INTO_ITER, <Iterator as From<Iterator>>::from)?;

    module.function_meta(range)?;
    module.function_meta(empty)?;
    module.function_meta(once)?;

    module.function_meta(collect_vec)?;
    module.function_meta(collect_vec_deque)?;
    module.function_meta(collect_hash_set)?;
    module.function_meta(collect_hash_map)?;
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
pub(crate) fn find(this: &mut Iterator, find: Function) -> VmResult<Option<Value>> {
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
pub fn any(this: &mut Iterator, find: Function) -> VmResult<bool> {
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
/// ```rune
/// let a = [1, 2, 3];
///
/// assert!(a.iter().all(|x| x > 0));
///
/// assert!(!a.iter().all(|x| x > 2));
/// ```
///
/// Stopping at the first `false`:
///
/// ```rune
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
pub fn all(this: &mut Iterator, find: Function) -> VmResult<bool> {
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
/// (0..5).iter().map(|x| println!("{}", x));
///
/// // it won't even execute, as it is lazy. Rust will warn you about this.
///
/// // Instead, use for:
/// for x in 0..5 {
///     println!("{}", x);
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

macro_rules! sum_ops {
    ($name:ident, $ty:ty) => {
        /// Sums the elements of an iterator.
        ///
        /// Takes each element, adds them together, and returns the result.
        ///
        /// An empty iterator returns the zero value of the type.
        ///
        /// `sum()` can be used to sum numerical built-in types, such as `int`, `float`
        /// and `u8`. The first element returned by the iterator determines the type
        /// being summed.
        ///
        /// # Panics
        ///
        /// When calling `sum()` and a primitive integer type is being returned, this
        /// method will panic if the computation overflows.
        ///
        /// # Examples
        ///
        /// Basic usage:
        ///
        /// ```rune
        #[doc = concat!(" let a = [1", stringify!($ty), ", 2", stringify!($ty), ", 3", stringify!($ty), "];")]
        #[doc = concat!(" let sum = a.iter().sum::<", stringify!($ty), ">();")]
        ///
        #[doc = concat!(" assert_eq!(sum, 6", stringify!($ty), ");")]
        /// ```
        #[rune::function(instance, path = sum::<$ty>)]
        #[inline]
        fn $name(this: Iterator) -> VmResult<$ty> {
            this.sum()
        }
    };
}

sum_ops!(sum_int, i64);
sum_ops!(sum_float, f64);
sum_ops!(sum_byte, u8);

macro_rules! integer_product_ops {
    ($name:ident, $ty:ty) => {
        /// Iterates over the entire iterator, multiplying all the elements
        ///
        /// An empty iterator returns the one value of the type.
        ///
        /// `sum()` can be used to sum numerical built-in types, such as `int`, `float`
        /// and `u8`. The first element returned by the iterator determines the type
        /// being multiplied.
        ///
        /// # Panics
        ///
        /// When calling `product()` and a primitive integer type is being returned,
        /// method will panic if the computation overflows.
        ///
        /// # Examples
        ///
        /// ```rune
        /// fn factorial(n) {
        #[doc = concat!("     (1", stringify!($ty), "..=n).iter().product::<", stringify!($ty), ">()")]
        /// }
        ///
        #[doc = concat!(" assert_eq!(factorial(0", stringify!($ty), "), 1", stringify!($ty), ");")]
        #[doc = concat!(" assert_eq!(factorial(1", stringify!($ty), "), 1", stringify!($ty), ");")]
        #[doc = concat!(" assert_eq!(factorial(5", stringify!($ty), "), 120", stringify!($ty), ");")]
        /// ```
        #[rune::function(instance, path = product::<$ty>)]
        #[inline]
        fn $name(this: Iterator) -> VmResult<$ty> {
            this.product::<$ty>()
        }
    };
}

macro_rules! float_product_ops {
    ($name:ident, $ty:ty) => {
        /// Iterates over the entire iterator, multiplying all the elements
        ///
        /// An empty iterator returns the one value of the type.
        ///
        /// `sum()` can be used to sum numerical built-in types, such as `int`, `float`
        /// and `u8`. The first element returned by the iterator determines the type
        /// being multiplied.
        ///
        /// # Panics
        ///
        /// When calling `product()` and a primitive integer type is being returned,
        /// method will panic if the computation overflows.
        ///
        /// # Examples
        ///
        /// ```rune
        /// fn factorial(n) {
        #[doc = concat!("     (1..=n).iter().map(|n| n as ", stringify!($ty), ").product::<", stringify!($ty), ">()")]
        /// }
        ///
        #[doc = concat!(" assert_eq!(factorial(0), 1", stringify!($ty), ");")]
        #[doc = concat!(" assert_eq!(factorial(1), 1", stringify!($ty), ");")]
        #[doc = concat!(" assert_eq!(factorial(5), 120", stringify!($ty), ");")]
        /// ```
        #[rune::function(instance, path = product::<$ty>)]
        #[inline]
        fn $name(this: Iterator) -> VmResult<$ty> {
            this.product::<$ty>()
        }
    };
}

integer_product_ops!(product_int, i64);
integer_product_ops!(product_byte, u8);
float_product_ops!(product_float, f64);

/// Folds every element into an accumulator by applying an operation, returning
/// the final result.
///
/// `fold()` takes two arguments: an initial value, and a closure with two
/// arguments: an 'accumulator', and an element. The closure returns the value
/// that the accumulator should have for the next iteration.
///
/// The initial value is the value the accumulator will have on the first call.
///
/// After applying this closure to every element of the iterator, `fold()`
/// returns the accumulator.
///
/// This operation is sometimes called 'reduce' or 'inject'.
///
/// Folding is useful whenever you have a collection of something, and want to
/// produce a single value from it.
///
/// Note: `fold()`, and similar methods that traverse the entire iterator, might
/// not terminate for infinite iterators, even on traits for which a result is
/// determinable in finite time.
///
/// Note: [`reduce()`] can be used to use the first element as the initial
/// value, if the accumulator type and item type is the same.
///
/// Note: `fold()` combines elements in a *left-associative* fashion. For
/// associative operators like `+`, the order the elements are combined in is
/// not important, but for non-associative operators like `-` the order will
/// affect the final result. For a *right-associative* version of `fold()`, see
/// [`DoubleEndedIterator::rfold()`].
///
/// # Note to Implementors
///
/// Several of the other (forward) methods have default implementations in
/// terms of this one, so try to implement this explicitly if it can
/// do something better than the default `for` loop implementation.
///
/// In particular, try to have this call `fold()` on the internal parts
/// from which this iterator is composed.
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// let a = [1, 2, 3];
///
/// // the sum of all of the elements of the array
/// let sum = a.iter().fold(0, |acc, x| acc + x);
///
/// assert_eq!(sum, 6);
/// ```
///
/// Let's walk through each step of the iteration here:
///
/// | element | acc | x | result |
/// |---------|-----|---|--------|
/// |         | 0   |   |        |
/// | 1       | 0   | 1 | 1      |
/// | 2       | 1   | 2 | 3      |
/// | 3       | 3   | 3 | 6      |
///
/// And so, our final result, `6`.
///
/// This example demonstrates the left-associative nature of `fold()`:
/// it builds a string, starting with an initial value
/// and continuing with each element from the front until the back:
///
/// ```rune
/// let numbers = [1, 2, 3, 4, 5];
///
/// let zero = "0";
///
/// let result = numbers.iter().fold(zero, |acc, x| {
///     format!("({} + {})", acc, x)
/// });
///
/// assert_eq!(result, "(((((0 + 1) + 2) + 3) + 4) + 5)");
/// ```
///
/// It's common for people who haven't used iterators a lot to
/// use a `for` loop with a list of things to build up a result. Those
/// can be turned into `fold()`s:
///
/// ```rune
/// let numbers = [1, 2, 3, 4, 5];
///
/// let result = 0;
///
/// // for loop:
/// for i in numbers {
///     result = result + i;
/// }
///
/// // fold:
/// let result2 = numbers.iter().fold(0, |acc, x| acc + x);
///
/// // they're the same
/// assert_eq!(result, result2);
/// ```
///
/// [`reduce()`]: Iterator::reduce
#[rune::function(instance)]
#[inline]
fn fold(this: Iterator, accumulator: Value, f: Function) -> VmResult<Value> {
    this.fold(accumulator, f)
}

/// Reduces the elements to a single one, by repeatedly applying a reducing
/// operation.
///
/// If the iterator is empty, returns [`None`]; otherwise, returns the result of
/// the reduction.
///
/// The reducing function is a closure with two arguments: an 'accumulator', and
/// an element. For iterators with at least one element, this is the same as
/// [`fold()`] with the first element of the iterator as the initial accumulator
/// value, folding every subsequent element into it.
///
/// [`fold()`]: Iterator::fold
///
/// # Example
///
/// ```rune
/// let reduced = (1..10).iter().reduce(|acc, e| acc + e).unwrap();
/// assert_eq!(reduced, 45);
///
/// // Which is equivalent to doing it with `fold`:
/// let folded = (1..10).iter().fold(0, |acc, e| acc + e);
/// assert_eq!(reduced, folded);
/// ```
#[rune::function(instance)]
#[inline]
fn reduce(this: Iterator, f: Function) -> VmResult<Option<Value>> {
    this.reduce(f)
}

/// Reverses an iterator's direction.
///
/// Usually, iterators iterate from left to right. After using `rev()`, an
/// iterator will instead iterate from right to left.
///
/// This is only possible if the iterator has an end, so `rev()` only works on
/// double-ended iterators.
///
/// # Examples
///
/// ```rune
/// let a = [1, 2, 3];
///
/// let iter = a.iter().rev();
///
/// assert_eq!(iter.next(), Some(3));
/// assert_eq!(iter.next(), Some(2));
/// assert_eq!(iter.next(), Some(1));
///
/// assert_eq!(iter.next(), None);
/// ```
#[rune::function(instance)]
#[inline]
fn rev(this: Iterator) -> VmResult<Iterator> {
    this.rev()
}

/// Returns the bounds on the remaining length of the iterator.
///
/// Specifically, `size_hint()` returns a tuple where the first element
/// is the lower bound, and the second element is the upper bound.
///
/// The second half of the tuple that is returned is an
/// <code>[Option]<[i64]></code>. A [`None`] here means that either there is no
/// known upper bound, or the upper bound is larger than [`i64`].
///
/// # Implementation notes
///
/// It is not enforced that an iterator implementation yields the declared
/// number of elements. A buggy iterator may yield less than the lower bound or
/// more than the upper bound of elements.
///
/// `size_hint()` is primarily intended to be used for optimizations such as
/// reserving space for the elements of the iterator, but must not be trusted to
/// e.g., omit bounds checks in unsafe code. An incorrect implementation of
/// `size_hint()` should not lead to memory safety violations.
///
/// That said, the implementation should provide a correct estimation, because
/// otherwise it would be a violation of the trait's protocol.
///
/// The default implementation returns <code>(0, [None])</code> which is correct
/// for any iterator.
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// let a = [1, 2, 3];
/// let iter = a.iter();
///
/// assert_eq!((3, Some(3)), iter.size_hint());
/// let _ = iter.next();
/// assert_eq!((2, Some(2)), iter.size_hint());
/// ```
///
/// A more complex example:
///
/// ```rune
/// // The even numbers in the range of zero to nine.
/// let iter = (0..10).iter().filter(|x| x % 2 == 0);
///
/// // We might iterate from zero to ten times. Knowing that it's five
/// // exactly wouldn't be possible without executing filter().
/// assert_eq!((0, Some(10)), iter.size_hint());
///
/// // Let's add five more numbers with chain()
/// let iter = (0..10).iter().filter(|x| x % 2 == 0).chain(15..20);
///
/// // now both bounds are increased by five
/// assert_eq!((5, Some(15)), iter.size_hint());
/// ```
///
/// Returning `None` for an upper bound:
///
/// ```rune
/// // an infinite iterator has no upper bound
/// // and the maximum possible lower bound
/// let iter = (0..).iter();
///
/// assert_eq!((i64::MAX, None), iter.size_hint());
/// ```
#[rune::function(instance)]
#[inline]
fn size_hint(this: &Iterator) -> (i64, Option<i64>) {
    let (lower, upper) = this.size_hint();
    let lower = i64::try_from(lower).unwrap_or(i64::MAX);
    let upper = upper.map(|upper| i64::try_from(upper).unwrap_or(i64::MAX));
    (lower, upper)
}

/// Creates an iterator that skips the first `n` elements.
///
/// `skip(n)` skips elements until `n` elements are skipped or the end of the
/// iterator is reached (whichever happens first). After that, all the remaining
/// elements are yielded. In particular, if the original iterator is too short,
/// then the returned iterator is empty.
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// let a = [1, 2, 3];
///
/// let iter = a.iter().skip(2);
///
/// assert_eq!(iter.next(), Some(3));
/// assert_eq!(iter.next(), None);
/// ```
#[rune::function(instance)]
#[inline]
fn skip(this: Iterator, n: usize) -> Iterator {
    this.skip(n)
}

/// Creates an iterator that yields the first `n` elements, or fewer if the
/// underlying iterator ends sooner.
///
/// `take(n)` yields elements until `n` elements are yielded or the end of the
/// iterator is reached (whichever happens first). The returned iterator is a
/// prefix of length `n` if the original iterator contains at least `n`
/// elements, otherwise it contains all of the (fewer than `n`) elements of the
/// original iterator.
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// let a = [1, 2, 3];
///
/// let iter = a.iter().take(2);
///
/// assert_eq!(iter.next(), Some(1));
/// assert_eq!(iter.next(), Some(2));
/// assert_eq!(iter.next(), None);
/// ```
///
/// `take()` is often used with an infinite iterator, to make it finite:
///
/// ```rune
/// let iter = (0..).iter().take(3);
///
/// assert_eq!(iter.next(), Some(0));
/// assert_eq!(iter.next(), Some(1));
/// assert_eq!(iter.next(), Some(2));
/// assert_eq!(iter.next(), None);
/// ```
///
/// If less than `n` elements are available, `take` will limit itself to the
/// size of the underlying iterator:
///
/// ```rune
/// let v = [1, 2];
/// let iter = v.iter().take(5);
/// assert_eq!(iter.next(), Some(1));
/// assert_eq!(iter.next(), Some(2));
/// assert_eq!(iter.next(), None);
/// ```
#[rune::function(instance)]
#[inline]
fn take(this: Iterator, n: usize) -> Iterator {
    this.take(n)
}

/// Consumes the iterator, counting the number of iterations and returning it.
///
/// This method will call [`next`] repeatedly until [`None`] is encountered,
/// returning the number of times it saw [`Some`]. Note that [`next`] has to be
/// called at least once even if the iterator does not have any elements.
///
/// [`next`]: Iterator::next
///
/// # Overflow Behavior
///
/// The method does no guarding against overflows, so counting elements of an
/// iterator with more than [`i64::MAX`] elements panics.
///
/// # Panics
///
/// This function might panic if the iterator has more than [`i64::MAX`]
/// elements.
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// let a = [1, 2, 3];
/// assert_eq!(a.iter().count(), 3);
///
/// let a = [1, 2, 3, 4, 5];
/// assert_eq!(a.iter().count(), 5);
/// ```
#[rune::function(instance)]
#[inline]
fn count(this: &mut Iterator) -> VmResult<usize> {
    this.count()
}

/// Collect the iterator as a [`Vec`].
///
/// # Examples
///
/// ```rune
/// use std::iter::range;
///
/// assert_eq!((0..3).iter().collect::<Vec>(), [0, 1, 2]);
/// ```
#[rune::function(instance, path = collect::<Vec>)]
fn collect_vec(it: Iterator) -> VmResult<Vec> {
    VmResult::Ok(Vec::from(vm_try!(it.collect::<Value>())))
}

/// Collect the iterator as a [`VecDeque`].
///
/// # Examples
///
/// ```rune
/// use std::collections::VecDeque;
///
/// assert_eq!((0..3).iter().collect::<VecDeque>(), VecDeque::from([0, 1, 2]));
/// ```
#[rune::function(instance, path = collect::<VecDeque>)]
fn collect_vec_deque(it: Iterator) -> VmResult<VecDeque> {
    VecDeque::from_iter(it)
}

/// Collect the iterator as a [`HashSet`].
///
/// # Examples
///
/// ```rune
/// use std::collections::HashSet;
///
/// assert_eq!((0..3).iter().collect::<HashSet>(), HashSet::from([0, 1, 2]));
/// ```
#[rune::function(instance, path = collect::<HashSet>)]
fn collect_hash_set(it: Iterator) -> VmResult<HashSet> {
    let mut caller = EnvProtocolCaller;
    HashSet::from_iter(it, &mut caller)
}

/// Collect the iterator as a [`HashMap`].
///
/// # Examples
///
/// ```rune
/// use std::collections::HashMap;
///
/// let actual = (0..3).iter().map(|n| (n, n.to_string())).collect::<HashMap>();
/// let expected = HashMap::from([(0, "0"), (1, "1"), (2, "2")]);
/// assert_eq!(actual, expected);
/// ```
#[rune::function(instance, path = collect::<HashMap>)]
fn collect_hash_map(it: Iterator) -> VmResult<HashMap> {
    let mut caller = EnvProtocolCaller;
    HashMap::from_iter(it, &mut caller)
}

/// Collect the iterator as a [`Tuple`].
///
/// # Examples
///
/// ```rune
/// assert_eq!((0..3).iter().collect::<Tuple>(), (0, 1, 2));
/// ```
#[rune::function(instance, path = collect::<OwnedTuple>)]
fn collect_tuple(it: Iterator) -> VmResult<OwnedTuple> {
    VmResult::Ok(vm_try!(OwnedTuple::try_from(
        vm_try!(it.collect::<Value>())
    )))
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
    let mut object = vm_try!(Object::with_capacity(cap));

    while let Some(value) = vm_try!(it.next()) {
        let (key, value) = vm_try!(<(String, Value)>::from_value(value));
        vm_try!(object.insert(key, value));
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
                vm_try!(string.try_push(c));
            }
            Value::String(s) => {
                let s = vm_try!(s.into_ref());
                vm_try!(string.try_push_str(s.as_str()));
            }
            value => {
                return VmResult::expected::<String>(vm_try!(value.type_info()));
            }
        }
    }

    VmResult::Ok(string)
}
