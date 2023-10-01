//! An ordered set based on a B-Tree.

use core::borrow::Borrow;
use core::cmp::Ordering::{self, Equal, Greater, Less};
use core::cmp::{max, min};
use core::fmt;
use core::hash::{Hash, Hasher};
use core::iter::{FusedIterator, Peekable};
use core::ops::RangeBounds;

use super::map::{infallible_cmp, into_ok, BTreeMap, CmpFn, Keys};
use super::merge_iter::MergeIterInner;
use super::set_val::SetValZST;
use super::Recover;

use crate::alloc::{AllocError, Allocator, Global};
use crate::clone::TryClone;
use crate::error::Error;
use crate::iter::{TryExtend, TryFromIteratorIn};
#[cfg(test)]
use crate::testing::*;

/// An ordered set based on a B-Tree.
///
/// See [`BTreeMap`]'s documentation for a detailed discussion of this collection's performance
/// benefits and drawbacks.
///
/// It is a logic error for an item to be modified in such a way that the item's ordering relative
/// to any other item, as determined by the [`Ord`] trait, changes while it is in the set. This is
/// normally only possible through [`Cell`], [`RefCell`], global state, I/O, or unsafe code.
/// The behavior resulting from such a logic error is not specified, but will be encapsulated to the
/// `BTreeSet` that observed the logic error and not result in undefined behavior. This could
/// include panics, incorrect results, aborts, memory leaks, and non-termination.
///
/// Iterators returned by [`BTreeSet::iter`] produce their items in order, and take worst-case
/// logarithmic and amortized constant time per item returned.
///
/// [`Cell`]: core::cell::Cell
/// [`RefCell`]: core::cell::RefCell
///
/// # Examples
///
/// ```
/// use rune::alloc::BTreeSet;
///
/// // Type inference lets us omit an explicit type signature (which
/// // would be `BTreeSet<&str>` in this example).
/// let mut books = BTreeSet::new();
///
/// // Add some books.
/// books.try_insert("A Dance With Dragons")?;
/// books.try_insert("To Kill a Mockingbird")?;
/// books.try_insert("The Odyssey")?;
/// books.try_insert("The Great Gatsby")?;
///
/// // Check for a specific one.
/// if !books.contains("The Winds of Winter") {
///     println!("We have {} books, but The Winds of Winter ain't one.",
///              books.len());
/// }
///
/// // Remove a book.
/// books.remove("The Odyssey");
///
/// // Iterate over everything.
/// for book in &books {
///     println!("{book}");
/// }
/// # Ok::<_, rune::alloc::Error>(())
/// ```
///
/// A `BTreeSet` with a known list of items can be initialized from an array:
///
/// ```
/// use rune::alloc::BTreeSet;
///
/// let set = BTreeSet::try_from([1, 2, 3])?;
/// # Ok::<_, rune::alloc::Error>(())
/// ```
pub struct BTreeSet<T, A: Allocator = Global> {
    map: BTreeMap<T, SetValZST, A>,
}

impl<T: Hash, A: Allocator> Hash for BTreeSet<T, A> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.map.hash(state)
    }
}

impl<T: PartialEq, A: Allocator> PartialEq for BTreeSet<T, A> {
    fn eq(&self, other: &BTreeSet<T, A>) -> bool {
        self.map.eq(&other.map)
    }
}

impl<T: Eq, A: Allocator> Eq for BTreeSet<T, A> {}

impl<T: PartialOrd, A: Allocator> PartialOrd for BTreeSet<T, A> {
    fn partial_cmp(&self, other: &BTreeSet<T, A>) -> Option<Ordering> {
        self.map.partial_cmp(&other.map)
    }
}

impl<T: Ord, A: Allocator> Ord for BTreeSet<T, A> {
    fn cmp(&self, other: &BTreeSet<T, A>) -> Ordering {
        self.map.cmp(&other.map)
    }
}

impl<T, A: Allocator + Clone> TryClone for BTreeSet<T, A>
where
    T: TryClone,
{
    fn try_clone(&self) -> Result<Self, Error> {
        Ok(BTreeSet {
            map: self.map.try_clone()?,
        })
    }
}

#[cfg(test)]
impl<T, A: Allocator + Clone> Clone for BTreeSet<T, A>
where
    T: TryClone,
{
    fn clone(&self) -> Self {
        self.try_clone().abort()
    }
}

/// An iterator over the items of a `BTreeSet`.
///
/// This `struct` is created by the [`iter`] method on [`BTreeSet`]. See its
/// documentation for more.
///
/// [`iter`]: BTreeSet::iter
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct Iter<'a, T: 'a> {
    iter: Keys<'a, T, SetValZST>,
}

impl<T> fmt::Debug for Iter<'_, T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Iter").field(&self.iter.clone()).finish()
    }
}

/// An owning iterator over the items of a `BTreeSet`.
///
/// This `struct` is created by the [`into_iter`] method on [`BTreeSet`]
/// (provided by the [`IntoIterator`] trait). See its documentation for more.
///
/// [`into_iter`]: BTreeSet#method.into_iter
#[derive(Debug)]
pub struct IntoIter<T, A: Allocator = Global> {
    iter: super::map::IntoIter<T, SetValZST, A>,
}

/// An iterator over a sub-range of items in a `BTreeSet`.
///
/// This `struct` is created by the [`range`] method on [`BTreeSet`].
/// See its documentation for more.
///
/// [`range`]: BTreeSet::range
#[must_use = "iterators are lazy and do nothing unless consumed"]
#[derive(Debug)]
pub struct Range<'a, T: 'a> {
    iter: super::map::Range<'a, T, SetValZST>,
}

/// A lazy iterator producing elements in the difference of `BTreeSet`s.
///
/// This `struct` is created by the [`difference`] method on [`BTreeSet`].
/// See its documentation for more.
///
/// [`difference`]: BTreeSet::difference
#[must_use = "this returns the difference as an iterator, \
              without modifying either input set"]
pub struct Difference<'a, T: 'a, A: Allocator = Global> {
    inner: DifferenceInner<'a, T, A>,
}

enum DifferenceInner<'a, T: 'a, A: Allocator> {
    Stitch {
        // iterate all of `self` and some of `other`, spotting matches along the way
        self_iter: Iter<'a, T>,
        other_iter: Peekable<Iter<'a, T>>,
    },
    Search {
        // iterate `self`, look up in `other`
        self_iter: Iter<'a, T>,
        other_set: &'a BTreeSet<T, A>,
    },
    Iterate(Iter<'a, T>), // simply produce all elements in `self`
}

// Explicit Debug impl necessary because of issue #26925
impl<T, A: Allocator> fmt::Debug for DifferenceInner<'_, T, A>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DifferenceInner::Stitch {
                self_iter,
                other_iter,
            } => f
                .debug_struct("Stitch")
                .field("self_iter", self_iter)
                .field("other_iter", other_iter)
                .finish(),
            DifferenceInner::Search {
                self_iter,
                other_set,
            } => f
                .debug_struct("Search")
                .field("self_iter", self_iter)
                .field("other_iter", other_set)
                .finish(),
            DifferenceInner::Iterate(x) => f.debug_tuple("Iterate").field(x).finish(),
        }
    }
}

impl<T, A: Allocator> fmt::Debug for Difference<'_, T, A>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Difference").field(&self.inner).finish()
    }
}

/// A lazy iterator producing elements in the symmetric difference of `BTreeSet`s.
///
/// This `struct` is created by the [`symmetric_difference`] method on
/// [`BTreeSet`]. See its documentation for more.
///
/// [`symmetric_difference`]: BTreeSet::symmetric_difference
#[must_use = "this returns the difference as an iterator, \
              without modifying either input set"]
pub struct SymmetricDifference<'a, T: 'a>(MergeIterInner<Iter<'a, T>>);

impl<T> fmt::Debug for SymmetricDifference<'_, T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("SymmetricDifference").field(&self.0).finish()
    }
}

/// A lazy iterator producing elements in the intersection of `BTreeSet`s.
///
/// This `struct` is created by the [`intersection`] method on [`BTreeSet`].
/// See its documentation for more.
///
/// [`intersection`]: BTreeSet::intersection
#[must_use = "this returns the intersection as an iterator, \
              without modifying either input set"]
pub struct Intersection<'a, T: 'a, A: Allocator = Global> {
    inner: IntersectionInner<'a, T, A>,
}

enum IntersectionInner<'a, T: 'a, A: Allocator> {
    Stitch {
        // iterate similarly sized sets jointly, spotting matches along the way
        a: Iter<'a, T>,
        b: Iter<'a, T>,
    },
    Search {
        // iterate a small set, look up in the large set
        small_iter: Iter<'a, T>,
        large_set: &'a BTreeSet<T, A>,
    },
    Answer(Option<&'a T>), // return a specific element or emptiness
}

// Explicit Debug impl necessary because of issue #26925
impl<T, A: Allocator> fmt::Debug for IntersectionInner<'_, T, A>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IntersectionInner::Stitch { a, b } => f
                .debug_struct("Stitch")
                .field("a", a)
                .field("b", b)
                .finish(),
            IntersectionInner::Search {
                small_iter,
                large_set,
            } => f
                .debug_struct("Search")
                .field("small_iter", small_iter)
                .field("large_set", large_set)
                .finish(),
            IntersectionInner::Answer(x) => f.debug_tuple("Answer").field(x).finish(),
        }
    }
}

impl<T, A: Allocator> fmt::Debug for Intersection<'_, T, A>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Intersection").field(&self.inner).finish()
    }
}

/// A lazy iterator producing elements in the union of `BTreeSet`s.
///
/// This `struct` is created by the [`union`] method on [`BTreeSet`].
/// See its documentation for more.
///
/// [`union`]: BTreeSet::union
#[must_use = "this returns the union as an iterator, \
              without modifying either input set"]
pub struct Union<'a, T: 'a>(MergeIterInner<Iter<'a, T>>);

impl<T> fmt::Debug for Union<'_, T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Union").field(&self.0).finish()
    }
}

// This constant is used by functions that compare two sets.
// It estimates the relative size at which searching performs better
// than iterating, based on the benchmarks in
// https://github.com/ssomers/rust_bench_btreeset_intersection.
// It's used to divide rather than multiply sizes, to rule out overflow,
// and it's a power of two to make that division cheap.
const ITER_PERFORMANCE_TIPPING_SIZE_DIFF: usize = 16;

impl<T> BTreeSet<T> {
    /// Makes a new, empty `BTreeSet`.
    ///
    /// Does not allocate anything on its own.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::BTreeSet;
    ///
    /// let mut set: BTreeSet<i32> = BTreeSet::new();
    /// ```
    #[must_use]
    pub const fn new() -> BTreeSet<T> {
        BTreeSet {
            map: BTreeMap::new(),
        }
    }

    #[cfg(test)]
    pub(crate) fn from<const N: usize>(values: [T; N]) -> Self
    where
        T: Ord,
    {
        Self::try_from(values).abort()
    }
}

impl<T, A: Allocator> BTreeSet<T, A> {
    /// Makes a new `BTreeSet` with a reasonable choice of B.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::BTreeSet;
    /// use rune::alloc::alloc::Global;
    ///
    /// let mut set: BTreeSet<i32> = BTreeSet::new_in(Global);
    /// ```
    pub fn new_in(alloc: A) -> BTreeSet<T, A> {
        BTreeSet {
            map: BTreeMap::new_in(alloc),
        }
    }

    /// Constructs a double-ended iterator over a sub-range of elements in the set.
    /// The simplest way is to use the range syntax `min..max`, thus `range(min..max)` will
    /// yield elements from min (inclusive) to max (exclusive).
    /// The range may also be entered as `(Bound<T>, Bound<T>)`, so for example
    /// `range((Excluded(4), Included(10)))` will yield a left-exclusive, right-inclusive
    /// range from 4 to 10.
    ///
    /// # Panics
    ///
    /// Panics if range `start > end`.
    /// Panics if range `start == end` and both bounds are `Excluded`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::BTreeSet;
    /// use std::ops::Bound::Included;
    ///
    /// let mut set = BTreeSet::new();
    /// set.try_insert(3)?;
    /// set.try_insert(5)?;
    /// set.try_insert(8)?;
    /// for &elem in set.range((Included(&4), Included(&8))) {
    ///     println!("{elem}");
    /// }
    /// assert_eq!(Some(&5), set.range(4..).next());
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn range<K: ?Sized, R>(&self, range: R) -> Range<'_, T>
    where
        K: Ord,
        T: Borrow<K> + Ord,
        R: RangeBounds<K>,
    {
        Range {
            iter: self.map.range(range),
        }
    }

    /// Visits the elements representing the difference,
    /// i.e., the elements that are in `self` but not in `other`,
    /// in ascending order.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::{BTreeSet, Vec};
    /// use rune::alloc::prelude::*;
    ///
    /// let mut a = BTreeSet::new();
    /// a.try_insert(1)?;
    /// a.try_insert(2)?;
    ///
    /// let mut b = BTreeSet::new();
    /// b.try_insert(2)?;
    /// b.try_insert(3)?;
    ///
    /// let diff: Vec<_> = a.difference(&b).cloned().try_collect()?;
    /// assert_eq!(diff, [1]);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn difference<'a>(&'a self, other: &'a BTreeSet<T, A>) -> Difference<'a, T, A>
    where
        T: Ord,
    {
        let (self_min, self_max) =
            if let (Some(self_min), Some(self_max)) = (self.first(), self.last()) {
                (self_min, self_max)
            } else {
                return Difference {
                    inner: DifferenceInner::Iterate(self.iter()),
                };
            };
        let (other_min, other_max) =
            if let (Some(other_min), Some(other_max)) = (other.first(), other.last()) {
                (other_min, other_max)
            } else {
                return Difference {
                    inner: DifferenceInner::Iterate(self.iter()),
                };
            };
        Difference {
            inner: match (self_min.cmp(other_max), self_max.cmp(other_min)) {
                (Greater, _) | (_, Less) => DifferenceInner::Iterate(self.iter()),
                (Equal, _) => {
                    let mut self_iter = self.iter();
                    self_iter.next();
                    DifferenceInner::Iterate(self_iter)
                }
                (_, Equal) => {
                    let mut self_iter = self.iter();
                    self_iter.next_back();
                    DifferenceInner::Iterate(self_iter)
                }
                _ if self.len() <= other.len() / ITER_PERFORMANCE_TIPPING_SIZE_DIFF => {
                    DifferenceInner::Search {
                        self_iter: self.iter(),
                        other_set: other,
                    }
                }
                _ => DifferenceInner::Stitch {
                    self_iter: self.iter(),
                    other_iter: other.iter().peekable(),
                },
            },
        }
    }

    /// Visits the elements representing the symmetric difference,
    /// i.e., the elements that are in `self` or in `other` but not in both,
    /// in ascending order.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::{BTreeSet, Vec};
    /// use rune::alloc::prelude::*;
    ///
    /// let mut a = BTreeSet::new();
    /// a.try_insert(1)?;
    /// a.try_insert(2)?;
    ///
    /// let mut b = BTreeSet::new();
    /// b.try_insert(2)?;
    /// b.try_insert(3)?;
    ///
    /// let sym_diff: Vec<_> = a.symmetric_difference(&b).cloned().try_collect()?;
    /// assert_eq!(sym_diff, [1, 3]);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn symmetric_difference<'a>(
        &'a self,
        other: &'a BTreeSet<T, A>,
    ) -> SymmetricDifference<'a, T>
    where
        T: Ord,
    {
        SymmetricDifference(MergeIterInner::new(self.iter(), other.iter()))
    }

    /// Visits the elements representing the intersection,
    /// i.e., the elements that are both in `self` and `other`,
    /// in ascending order.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::{BTreeSet, Vec};
    /// use rune::alloc::prelude::*;
    ///
    /// let mut a = BTreeSet::new();
    /// a.try_insert(1)?;
    /// a.try_insert(2)?;
    ///
    /// let mut b = BTreeSet::new();
    /// b.try_insert(2)?;
    /// b.try_insert(3)?;
    ///
    /// let intersection: Vec<_> = a.intersection(&b).cloned().try_collect()?;
    /// assert_eq!(intersection, [2]);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn intersection<'a>(&'a self, other: &'a BTreeSet<T, A>) -> Intersection<'a, T, A>
    where
        T: Ord,
    {
        let (self_min, self_max) =
            if let (Some(self_min), Some(self_max)) = (self.first(), self.last()) {
                (self_min, self_max)
            } else {
                return Intersection {
                    inner: IntersectionInner::Answer(None),
                };
            };
        let (other_min, other_max) =
            if let (Some(other_min), Some(other_max)) = (other.first(), other.last()) {
                (other_min, other_max)
            } else {
                return Intersection {
                    inner: IntersectionInner::Answer(None),
                };
            };
        Intersection {
            inner: match (self_min.cmp(other_max), self_max.cmp(other_min)) {
                (Greater, _) | (_, Less) => IntersectionInner::Answer(None),
                (Equal, _) => IntersectionInner::Answer(Some(self_min)),
                (_, Equal) => IntersectionInner::Answer(Some(self_max)),
                _ if self.len() <= other.len() / ITER_PERFORMANCE_TIPPING_SIZE_DIFF => {
                    IntersectionInner::Search {
                        small_iter: self.iter(),
                        large_set: other,
                    }
                }
                _ if other.len() <= self.len() / ITER_PERFORMANCE_TIPPING_SIZE_DIFF => {
                    IntersectionInner::Search {
                        small_iter: other.iter(),
                        large_set: self,
                    }
                }
                _ => IntersectionInner::Stitch {
                    a: self.iter(),
                    b: other.iter(),
                },
            },
        }
    }

    /// Visits the elements representing the union,
    /// i.e., all the elements in `self` or `other`, without duplicates,
    /// in ascending order.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::{BTreeSet, Vec};
    /// use rune::alloc::prelude::*;
    ///
    /// let mut a = BTreeSet::new();
    /// a.try_insert(1)?;
    ///
    /// let mut b = BTreeSet::new();
    /// b.try_insert(2)?;
    ///
    /// let union: Vec<_> = a.union(&b).cloned().try_collect()?;
    /// assert_eq!(union, [1, 2]);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn union<'a>(&'a self, other: &'a BTreeSet<T, A>) -> Union<'a, T>
    where
        T: Ord,
    {
        Union(MergeIterInner::new(self.iter(), other.iter()))
    }

    /// Clears the set, removing all elements.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::{BTreeSet, Vec};
    /// use rune::alloc::prelude::*;
    ///
    /// let mut v = BTreeSet::new();
    /// v.try_insert(1)?;
    /// v.clear();
    /// assert!(v.is_empty());
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn clear(&mut self) {
        self.map.clear()
    }

    /// Returns `true` if the set contains an element equal to the value.
    ///
    /// The value may be any borrowed form of the set's element type,
    /// but the ordering on the borrowed form *must* match the
    /// ordering on the element type.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::BTreeSet;
    ///
    /// let set = BTreeSet::try_from([1, 2, 3])?;
    /// assert_eq!(set.contains(&1), true);
    /// assert_eq!(set.contains(&4), false);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn contains<Q: ?Sized>(&self, value: &Q) -> bool
    where
        T: Borrow<Q> + Ord,
        Q: Ord,
    {
        self.map.contains_key(value)
    }

    /// Returns a reference to the element in the set, if any, that is equal to
    /// the value.
    ///
    /// The value may be any borrowed form of the set's element type,
    /// but the ordering on the borrowed form *must* match the
    /// ordering on the element type.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::BTreeSet;
    ///
    /// let set = BTreeSet::try_from([1, 2, 3])?;
    /// assert_eq!(set.get(&2), Some(&2));
    /// assert_eq!(set.get(&4), None);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn get<Q: ?Sized>(&self, value: &Q) -> Option<&T>
    where
        T: Borrow<Q> + Ord,
        Q: Ord,
    {
        into_ok(self.get_with(&mut (), value, infallible_cmp))
    }

    pub(crate) fn get_with<C: ?Sized, Q: ?Sized, E>(
        &self,
        cx: &mut C,
        value: &Q,
        cmp: CmpFn<C, Q, E>,
    ) -> Result<Option<&T>, E>
    where
        T: Borrow<Q>,
    {
        Recover::get(&self.map, cx, value, cmp)
    }

    /// Returns `true` if `self` has no elements in common with `other`. This is
    /// equivalent to checking for an empty intersection.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::BTreeSet;
    ///
    /// let a = BTreeSet::try_from([1, 2, 3])?;
    /// let mut b = BTreeSet::new();
    ///
    /// assert_eq!(a.is_disjoint(&b), true);
    /// b.try_insert(4)?;
    /// assert_eq!(a.is_disjoint(&b), true);
    /// b.try_insert(1)?;
    /// assert_eq!(a.is_disjoint(&b), false);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[must_use]
    pub fn is_disjoint(&self, other: &BTreeSet<T, A>) -> bool
    where
        T: Ord,
    {
        self.intersection(other).next().is_none()
    }

    /// Returns `true` if the set is a subset of another,
    /// i.e., `other` contains at least all the elements in `self`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::BTreeSet;
    ///
    /// let sup = BTreeSet::try_from([1, 2, 3])?;
    /// let mut set = BTreeSet::new();
    ///
    /// assert_eq!(set.is_subset(&sup), true);
    /// set.try_insert(2)?;
    /// assert_eq!(set.is_subset(&sup), true);
    /// set.try_insert(4)?;
    /// assert_eq!(set.is_subset(&sup), false);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[must_use]
    pub fn is_subset(&self, other: &BTreeSet<T, A>) -> bool
    where
        T: Ord,
    {
        // Same result as self.difference(other).next().is_none()
        // but the code below is faster (hugely in some cases).
        if self.len() > other.len() {
            return false;
        }
        let (self_min, self_max) =
            if let (Some(self_min), Some(self_max)) = (self.first(), self.last()) {
                (self_min, self_max)
            } else {
                return true; // self is empty
            };
        let (other_min, other_max) =
            if let (Some(other_min), Some(other_max)) = (other.first(), other.last()) {
                (other_min, other_max)
            } else {
                return false; // other is empty
            };
        let mut self_iter = self.iter();
        match self_min.cmp(other_min) {
            Less => return false,
            Equal => {
                self_iter.next();
            }
            Greater => (),
        }
        match self_max.cmp(other_max) {
            Greater => return false,
            Equal => {
                self_iter.next_back();
            }
            Less => (),
        }
        if self_iter.len() <= other.len() / ITER_PERFORMANCE_TIPPING_SIZE_DIFF {
            for next in self_iter {
                if !other.contains(next) {
                    return false;
                }
            }
        } else {
            let mut other_iter = other.iter();
            other_iter.next();
            other_iter.next_back();
            let mut self_next = self_iter.next();
            while let Some(self1) = self_next {
                match other_iter.next().map_or(Less, |other1| self1.cmp(other1)) {
                    Less => return false,
                    Equal => self_next = self_iter.next(),
                    Greater => (),
                }
            }
        }
        true
    }

    /// Returns `true` if the set is a superset of another,
    /// i.e., `self` contains at least all the elements in `other`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::BTreeSet;
    ///
    /// let sub = BTreeSet::try_from([1, 2])?;
    /// let mut set = BTreeSet::new();
    ///
    /// assert_eq!(set.is_superset(&sub), false);
    ///
    /// set.try_insert(0)?;
    /// set.try_insert(1)?;
    /// assert_eq!(set.is_superset(&sub), false);
    ///
    /// set.try_insert(2)?;
    /// assert_eq!(set.is_superset(&sub), true);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[must_use]
    pub fn is_superset(&self, other: &BTreeSet<T, A>) -> bool
    where
        T: Ord,
    {
        other.is_subset(self)
    }

    /// Returns a reference to the first element in the set, if any.
    /// This element is always the minimum of all elements in the set.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use rune::alloc::BTreeSet;
    ///
    /// let mut set = BTreeSet::new();
    /// assert_eq!(set.first(), None);
    /// set.try_insert(1)?;
    /// assert_eq!(set.first(), Some(&1));
    /// set.try_insert(2)?;
    /// assert_eq!(set.first(), Some(&1));
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[must_use]
    pub fn first(&self) -> Option<&T>
    where
        T: Ord,
    {
        self.map.first_key_value().map(|(k, _)| k)
    }

    /// Returns a reference to the last element in the set, if any.
    /// This element is always the maximum of all elements in the set.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use rune::alloc::BTreeSet;
    ///
    /// let mut set = BTreeSet::new();
    /// assert_eq!(set.last(), None);
    /// set.try_insert(1)?;
    /// assert_eq!(set.last(), Some(&1));
    /// set.try_insert(2)?;
    /// assert_eq!(set.last(), Some(&2));
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[must_use]
    pub fn last(&self) -> Option<&T>
    where
        T: Ord,
    {
        self.map.last_key_value().map(|(k, _)| k)
    }

    /// Removes the first element from the set and returns it, if any.
    /// The first element is always the minimum element in the set.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::BTreeSet;
    ///
    /// let mut set = BTreeSet::new();
    ///
    /// set.try_insert(1)?;
    ///
    /// while let Some(n) = set.pop_first() {
    ///     assert_eq!(n, 1);
    /// }
    ///
    /// assert!(set.is_empty());
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn pop_first(&mut self) -> Option<T>
    where
        T: Ord,
    {
        self.map.pop_first().map(|kv| kv.0)
    }

    /// Removes the last element from the set and returns it, if any. The last
    /// element is always the maximum element in the set.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::BTreeSet;
    ///
    /// let mut set = BTreeSet::new();
    ///
    /// set.try_insert(1)?;
    ///
    /// while let Some(n) = set.pop_last() {
    ///     assert_eq!(n, 1);
    /// }
    ///
    /// assert!(set.is_empty());
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn pop_last(&mut self) -> Option<T>
    where
        T: Ord,
    {
        self.map.pop_last().map(|kv| kv.0)
    }

    /// Adds a value to the set.
    ///
    /// Returns whether the value was newly inserted. That is:
    ///
    /// - If the set did not previously contain an equal value, `true` is
    ///   returned.
    /// - If the set already contained an equal value, `false` is returned, and
    ///   the entry is not updated.
    ///
    /// See the [module-level documentation] for more.
    ///
    /// [module-level documentation]: index.html#insert-and-complex-keys
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::BTreeSet;
    ///
    /// let mut set = BTreeSet::new();
    ///
    /// assert_eq!(set.try_insert(2)?, true);
    /// assert_eq!(set.try_insert(2)?, false);
    /// assert_eq!(set.len(), 1);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn try_insert(&mut self, value: T) -> Result<bool, AllocError>
    where
        T: Ord,
    {
        Ok(self.map.try_insert(value, SetValZST)?.is_none())
    }

    #[cfg(test)]
    pub(crate) fn insert(&mut self, value: T) -> bool
    where
        T: Ord,
    {
        self.try_insert(value).abort()
    }

    /// Adds a value to the set, replacing the existing element, if any, that is
    /// equal to the value. Returns the replaced element.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::{Vec, BTreeSet};
    ///
    /// let mut set = BTreeSet::new();
    /// set.try_insert(Vec::<i32>::new())?;
    ///
    /// assert_eq!(set.get(&[][..]).unwrap().capacity(), 0);
    /// set.try_replace(Vec::try_with_capacity(10)?)?;
    /// assert_eq!(set.get(&[][..]).unwrap().capacity(), 10);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn try_replace(&mut self, value: T) -> Result<Option<T>, AllocError>
    where
        T: Ord,
    {
        into_ok(self.try_replace_with(&mut (), value, infallible_cmp))
    }

    #[cfg(test)]
    pub(crate) fn replace(&mut self, value: T) -> Option<T>
    where
        T: Ord,
    {
        self.try_replace(value).abort()
    }

    pub(crate) fn try_replace_with<C: ?Sized, E>(
        &mut self,
        cx: &mut C,
        value: T,
        cmp: CmpFn<C, T, E>,
    ) -> Result<Result<Option<T>, AllocError>, E> {
        Recover::try_replace(&mut self.map, cx, value, cmp)
    }

    /// If the set contains an element equal to the value, removes it from the
    /// set and drops it. Returns whether such an element was present.
    ///
    /// The value may be any borrowed form of the set's element type,
    /// but the ordering on the borrowed form *must* match the
    /// ordering on the element type.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::BTreeSet;
    ///
    /// let mut set = BTreeSet::new();
    ///
    /// set.try_insert(2)?;
    /// assert_eq!(set.remove(&2), true);
    /// assert_eq!(set.remove(&2), false);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn remove<Q: ?Sized>(&mut self, value: &Q) -> bool
    where
        T: Borrow<Q> + Ord,
        Q: Ord,
    {
        self.map.remove(value).is_some()
    }

    /// Removes and returns the element in the set, if any, that is equal to
    /// the value.
    ///
    /// The value may be any borrowed form of the set's element type,
    /// but the ordering on the borrowed form *must* match the
    /// ordering on the element type.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::BTreeSet;
    ///
    /// let mut set = BTreeSet::try_from([1, 2, 3])?;
    /// assert_eq!(set.take(&2), Some(2));
    /// assert_eq!(set.take(&2), None);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn take<Q: ?Sized>(&mut self, value: &Q) -> Option<T>
    where
        T: Borrow<Q> + Ord,
        Q: Ord,
    {
        into_ok(self.take_with(&mut (), value, infallible_cmp))
    }

    pub(crate) fn take_with<C: ?Sized, Q: ?Sized, E>(
        &mut self,
        cx: &mut C,
        value: &Q,
        cmp: CmpFn<C, Q, E>,
    ) -> Result<Option<T>, E>
    where
        T: Borrow<Q>,
    {
        Recover::take(&mut self.map, cx, value, cmp)
    }

    /// Retains only the elements specified by the predicate.
    ///
    /// In other words, remove all elements `e` for which `f(&e)` returns `false`.
    /// The elements are visited in ascending order.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::BTreeSet;
    ///
    /// let mut set = BTreeSet::try_from([1, 2, 3, 4, 5, 6])?;
    /// // Keep only the even numbers.
    /// set.retain(|&k| k % 2 == 0);
    /// assert!(set.iter().eq([2, 4, 6].iter()));
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn retain<F>(&mut self, mut f: F)
    where
        T: Ord,
        F: FnMut(&T) -> bool,
    {
        self.extract_if(|v| !f(v)).for_each(drop);
    }

    /// Moves all elements from `other` into `self`, leaving `other` empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::BTreeSet;
    ///
    /// let mut a = BTreeSet::new();
    /// a.try_insert(1)?;
    /// a.try_insert(2)?;
    /// a.try_insert(3)?;
    ///
    /// let mut b = BTreeSet::new();
    /// b.try_insert(3)?;
    /// b.try_insert(4)?;
    /// b.try_insert(5)?;
    ///
    /// a.try_append(&mut b)?;
    ///
    /// assert_eq!(a.len(), 5);
    /// assert_eq!(b.len(), 0);
    ///
    /// assert!(a.contains(&1));
    /// assert!(a.contains(&2));
    /// assert!(a.contains(&3));
    /// assert!(a.contains(&4));
    /// assert!(a.contains(&5));
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn try_append(&mut self, other: &mut Self) -> Result<(), AllocError>
    where
        T: Ord,
    {
        self.map.try_append(&mut other.map)
    }

    #[cfg(test)]
    pub(crate) fn append(&mut self, other: &mut Self)
    where
        T: Ord,
    {
        self.try_append(other).abort()
    }

    /// Splits the collection into two at the value. Returns a new collection
    /// with all elements greater than or equal to the value.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use rune::alloc::BTreeSet;
    ///
    /// let mut a = BTreeSet::new();
    /// a.try_insert(1)?;
    /// a.try_insert(2)?;
    /// a.try_insert(3)?;
    /// a.try_insert(17)?;
    /// a.try_insert(41)?;
    ///
    /// let b = a.try_split_off(&3)?;
    ///
    /// assert_eq!(a.len(), 2);
    /// assert_eq!(b.len(), 3);
    ///
    /// assert!(a.contains(&1));
    /// assert!(a.contains(&2));
    ///
    /// assert!(b.contains(&3));
    /// assert!(b.contains(&17));
    /// assert!(b.contains(&41));
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn try_split_off<Q: ?Sized + Ord>(&mut self, value: &Q) -> Result<Self, Error>
    where
        T: Borrow<Q> + Ord,
        A: Clone,
    {
        Ok(BTreeSet {
            map: self.map.try_split_off(value)?,
        })
    }

    #[cfg(test)]
    pub(crate) fn split_off<Q: ?Sized + Ord>(&mut self, value: &Q) -> Self
    where
        T: Borrow<Q> + Ord,
        A: Clone,
    {
        self.try_split_off(value).abort()
    }

    /// Creates an iterator that visits all elements in ascending order and
    /// uses a closure to determine if an element should be removed.
    ///
    /// If the closure returns `true`, the element is removed from the set and
    /// yielded. If the closure returns `false`, or panics, the element remains
    /// in the set and will not be yielded.
    ///
    /// If the returned `ExtractIf` is not exhausted, e.g. because it is dropped without iterating
    /// or the iteration short-circuits, then the remaining elements will be retained.
    /// Use [`retain`] with a negated predicate if you do not need the returned iterator.
    ///
    /// [`retain`]: BTreeSet::retain
    /// # Examples
    ///
    /// Splitting a set into even and odd values, reusing the original set:
    ///
    /// ```
    /// use rune::alloc::{BTreeSet, Vec};
    /// use rune::alloc::prelude::*;
    ///
    /// let mut set: BTreeSet<i32> = (0..8).try_collect()?;
    /// let evens: BTreeSet<_> = set.extract_if(|v| v % 2 == 0).try_collect()?;
    /// let odds = set;
    /// assert_eq!(evens.into_iter().try_collect::<Vec<_>>()?, [0, 2, 4, 6]);
    /// assert_eq!(odds.into_iter().try_collect::<Vec<_>>()?, [1, 3, 5, 7]);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn extract_if<'a, F>(&'a mut self, pred: F) -> ExtractIf<'a, T, F, A>
    where
        T: Ord,
        F: 'a + FnMut(&T) -> bool,
    {
        let (inner, alloc) = self.map.extract_if_inner();
        ExtractIf { pred, inner, alloc }
    }

    /// Gets an iterator that visits the elements in the `BTreeSet` in ascending
    /// order.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::BTreeSet;
    ///
    /// let set = BTreeSet::try_from([1, 2, 3])?;
    /// let mut set_iter = set.iter();
    /// assert_eq!(set_iter.next(), Some(&1));
    /// assert_eq!(set_iter.next(), Some(&2));
    /// assert_eq!(set_iter.next(), Some(&3));
    /// assert_eq!(set_iter.next(), None);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    ///
    /// Values returned by the iterator are returned in ascending order:
    ///
    /// ```
    /// use rune::alloc::BTreeSet;
    ///
    /// let set = BTreeSet::try_from([3, 1, 2])?;
    /// let mut set_iter = set.iter();
    /// assert_eq!(set_iter.next(), Some(&1));
    /// assert_eq!(set_iter.next(), Some(&2));
    /// assert_eq!(set_iter.next(), Some(&3));
    /// assert_eq!(set_iter.next(), None);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn iter(&self) -> Iter<'_, T> {
        Iter {
            iter: self.map.keys(),
        }
    }

    /// Returns the number of elements in the set.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::BTreeSet;
    ///
    /// let mut v = BTreeSet::new();
    /// assert_eq!(v.len(), 0);
    /// v.try_insert(1)?;
    /// assert_eq!(v.len(), 1);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[must_use]
    pub const fn len(&self) -> usize {
        self.map.len()
    }

    /// Returns `true` if the set contains no elements.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::BTreeSet;
    ///
    /// let mut v = BTreeSet::new();
    /// assert!(v.is_empty());
    /// v.try_insert(1)?;
    /// assert!(!v.is_empty());
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<T, A: Allocator> IntoIterator for BTreeSet<T, A> {
    type Item = T;
    type IntoIter = IntoIter<T, A>;

    /// Gets an iterator for moving out the `BTreeSet`'s contents.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::{BTreeSet, Vec};
    /// use rune::alloc::prelude::*;
    ///
    /// let set = BTreeSet::try_from([1, 2, 3, 4])?;
    ///
    /// let v: Vec<_> = set.into_iter().try_collect()?;
    /// assert_eq!(v, [1, 2, 3, 4]);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    fn into_iter(self) -> IntoIter<T, A> {
        IntoIter {
            iter: self.map.into_iter(),
        }
    }
}

impl<'a, T, A: Allocator> IntoIterator for &'a BTreeSet<T, A> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Iter<'a, T> {
        self.iter()
    }
}

/// An iterator produced by calling `extract_if` on BTreeSet.
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct ExtractIf<'a, T, F, A: Allocator = Global>
where
    T: 'a,
    F: 'a + FnMut(&T) -> bool,
{
    pred: F,
    inner: super::map::ExtractIfInner<'a, T, SetValZST>,
    /// The BTreeMap will outlive this IntoIter so we don't care about drop order for `alloc`.
    alloc: &'a A,
}

impl<T, F, A: Allocator> fmt::Debug for ExtractIf<'_, T, F, A>
where
    T: fmt::Debug,
    F: FnMut(&T) -> bool,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ExtractIf")
            .field(&self.inner.peek().map(|(k, _)| k))
            .finish()
    }
}

impl<'a, T, F, A: Allocator> Iterator for ExtractIf<'_, T, F, A>
where
    F: 'a + FnMut(&T) -> bool,
{
    type Item = T;

    fn next(&mut self) -> Option<T> {
        let pred = &mut self.pred;
        let mut mapped_pred = |k: &T, _v: &mut SetValZST| pred(k);
        self.inner
            .next(&mut mapped_pred, self.alloc)
            .map(|(k, _)| k)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<T, F, A: Allocator> FusedIterator for ExtractIf<'_, T, F, A> where F: FnMut(&T) -> bool {}

impl<T, A: Allocator> TryExtend<T> for BTreeSet<T, A>
where
    T: Ord,
{
    #[inline]
    fn try_extend<Iter: IntoIterator<Item = T>>(&mut self, iter: Iter) -> Result<(), Error> {
        for elem in iter {
            self.try_insert(elem)?;
        }

        Ok(())
    }
}

#[cfg(test)]
impl<T, A: Allocator> Extend<T> for BTreeSet<T, A>
where
    T: Ord,
{
    #[inline]
    fn extend<Iter: IntoIterator<Item = T>>(&mut self, iter: Iter) {
        self.try_extend(iter).abort()
    }
}

impl<'a, T, A: Allocator> TryExtend<&'a T> for BTreeSet<T, A>
where
    T: 'a + Ord + Copy,
{
    fn try_extend<I: IntoIterator<Item = &'a T>>(&mut self, iter: I) -> Result<(), Error> {
        self.try_extend(iter.into_iter().copied())
    }
}

#[cfg(test)]
impl<'a, T, A: Allocator> Extend<&'a T> for BTreeSet<T, A>
where
    T: 'a + Ord + Copy,
{
    fn extend<I: IntoIterator<Item = &'a T>>(&mut self, iter: I) {
        self.try_extend(iter).abort()
    }
}

impl<T> Default for BTreeSet<T> {
    /// Creates an empty `BTreeSet`.
    fn default() -> BTreeSet<T> {
        BTreeSet::new()
    }
}

impl<T, A: Allocator> fmt::Debug for BTreeSet<T, A>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

impl<T> Clone for Iter<'_, T> {
    fn clone(&self) -> Self {
        Iter {
            iter: self.iter.clone(),
        }
    }
}
impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        self.iter.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }

    fn last(mut self) -> Option<&'a T> {
        self.next_back()
    }

    fn min(mut self) -> Option<&'a T>
    where
        &'a T: Ord,
    {
        self.next()
    }

    fn max(mut self) -> Option<&'a T>
    where
        &'a T: Ord,
    {
        self.next_back()
    }
}

impl<'a, T> DoubleEndedIterator for Iter<'a, T> {
    fn next_back(&mut self) -> Option<&'a T> {
        self.iter.next_back()
    }
}
impl<T> ExactSizeIterator for Iter<'_, T> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl<T> FusedIterator for Iter<'_, T> {}

impl<T, A: Allocator> Iterator for IntoIter<T, A> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        self.iter.next().map(|(k, _)| k)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<T> Default for Iter<'_, T> {
    /// Creates an empty `btree_set::Iter`.
    ///
    /// ```
    /// use rune::alloc::btree_set;
    ///
    /// let iter: btree_set::Iter<'_, u8> = Default::default();
    /// assert_eq!(iter.len(), 0);
    /// ```
    fn default() -> Self {
        Iter {
            iter: Default::default(),
        }
    }
}

impl<T, A: Allocator> DoubleEndedIterator for IntoIter<T, A> {
    fn next_back(&mut self) -> Option<T> {
        self.iter.next_back().map(|(k, _)| k)
    }
}
impl<T, A: Allocator> ExactSizeIterator for IntoIter<T, A> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl<T, A: Allocator> FusedIterator for IntoIter<T, A> {}

impl<T, A> Default for IntoIter<T, A>
where
    A: Allocator + Default + Clone,
{
    /// Creates an empty `btree_set::IntoIter`.
    ///
    /// ```
    /// use rune::alloc::btree_set;
    ///
    /// let iter: btree_set::IntoIter<u8> = Default::default();
    /// assert_eq!(iter.len(), 0);
    /// ```
    fn default() -> Self {
        IntoIter {
            iter: Default::default(),
        }
    }
}

impl<T> Clone for Range<'_, T> {
    fn clone(&self) -> Self {
        Range {
            iter: self.iter.clone(),
        }
    }
}

impl<'a, T> Iterator for Range<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        self.iter.next().map(|(k, _)| k)
    }

    fn last(mut self) -> Option<&'a T> {
        self.next_back()
    }

    fn min(mut self) -> Option<&'a T>
    where
        &'a T: Ord,
    {
        self.next()
    }

    fn max(mut self) -> Option<&'a T>
    where
        &'a T: Ord,
    {
        self.next_back()
    }
}

impl<'a, T> DoubleEndedIterator for Range<'a, T> {
    fn next_back(&mut self) -> Option<&'a T> {
        self.iter.next_back().map(|(k, _)| k)
    }
}

impl<T> FusedIterator for Range<'_, T> {}

impl<T> Default for Range<'_, T> {
    /// Creates an empty `btree_set::Range`.
    ///
    /// ```
    /// use rune::alloc::btree_set;
    ///
    /// let iter: btree_set::Range<'_, u8> = Default::default();
    /// assert_eq!(iter.count(), 0);
    /// ```
    fn default() -> Self {
        Range {
            iter: Default::default(),
        }
    }
}

impl<T, A: Allocator> Clone for Difference<'_, T, A> {
    fn clone(&self) -> Self {
        Difference {
            inner: match &self.inner {
                DifferenceInner::Stitch {
                    self_iter,
                    other_iter,
                } => DifferenceInner::Stitch {
                    self_iter: self_iter.clone(),
                    other_iter: other_iter.clone(),
                },
                DifferenceInner::Search {
                    self_iter,
                    other_set,
                } => DifferenceInner::Search {
                    self_iter: self_iter.clone(),
                    other_set,
                },
                DifferenceInner::Iterate(iter) => DifferenceInner::Iterate(iter.clone()),
            },
        }
    }
}

impl<'a, T: Ord, A: Allocator> Iterator for Difference<'a, T, A> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        match &mut self.inner {
            DifferenceInner::Stitch {
                self_iter,
                other_iter,
            } => {
                let mut self_next = self_iter.next()?;

                loop {
                    match other_iter
                        .peek()
                        .map_or(Less, |other_next| self_next.cmp(other_next))
                    {
                        Less => return Some(self_next),
                        Equal => {
                            self_next = self_iter.next()?;
                            other_iter.next();
                        }
                        Greater => {
                            other_iter.next();
                        }
                    }
                }
            }
            DifferenceInner::Search {
                self_iter,
                other_set,
            } => loop {
                let self_next = self_iter.next()?;

                if !other_set.contains(self_next) {
                    return Some(self_next);
                }
            },
            DifferenceInner::Iterate(iter) => iter.next(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (self_len, other_len) = match &self.inner {
            DifferenceInner::Stitch {
                self_iter,
                other_iter,
            } => (self_iter.len(), other_iter.len()),
            DifferenceInner::Search {
                self_iter,
                other_set,
            } => (self_iter.len(), other_set.len()),
            DifferenceInner::Iterate(iter) => (iter.len(), 0),
        };
        (self_len.saturating_sub(other_len), Some(self_len))
    }

    fn min(mut self) -> Option<&'a T> {
        self.next()
    }
}

impl<T: Ord, A: Allocator> FusedIterator for Difference<'_, T, A> {}

impl<T> Clone for SymmetricDifference<'_, T> {
    fn clone(&self) -> Self {
        SymmetricDifference(self.0.clone())
    }
}

impl<'a, T: Ord> Iterator for SymmetricDifference<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        loop {
            let (a_next, b_next) = self.0.nexts(Self::Item::cmp);
            if a_next.and(b_next).is_none() {
                return a_next.or(b_next);
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (a_len, b_len) = self.0.lens();
        // No checked_add, because even if a and b refer to the same set,
        // and T is a zero-sized type, the storage overhead of sets limits
        // the number of elements to less than half the range of usize.
        (0, Some(a_len + b_len))
    }

    fn min(mut self) -> Option<&'a T> {
        self.next()
    }
}

impl<T: Ord> FusedIterator for SymmetricDifference<'_, T> {}

impl<T, A: Allocator> Clone for Intersection<'_, T, A> {
    fn clone(&self) -> Self {
        Intersection {
            inner: match &self.inner {
                IntersectionInner::Stitch { a, b } => IntersectionInner::Stitch {
                    a: a.clone(),
                    b: b.clone(),
                },
                IntersectionInner::Search {
                    small_iter,
                    large_set,
                } => IntersectionInner::Search {
                    small_iter: small_iter.clone(),
                    large_set,
                },
                IntersectionInner::Answer(answer) => IntersectionInner::Answer(*answer),
            },
        }
    }
}
impl<'a, T: Ord, A: Allocator> Iterator for Intersection<'a, T, A> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        match &mut self.inner {
            IntersectionInner::Stitch { a, b } => {
                let mut a_next = a.next()?;
                let mut b_next = b.next()?;
                loop {
                    match a_next.cmp(b_next) {
                        Less => a_next = a.next()?,
                        Greater => b_next = b.next()?,
                        Equal => return Some(a_next),
                    }
                }
            }
            IntersectionInner::Search {
                small_iter,
                large_set,
            } => loop {
                let small_next = small_iter.next()?;
                if large_set.contains(small_next) {
                    return Some(small_next);
                }
            },
            IntersectionInner::Answer(answer) => answer.take(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.inner {
            IntersectionInner::Stitch { a, b } => (0, Some(min(a.len(), b.len()))),
            IntersectionInner::Search { small_iter, .. } => (0, Some(small_iter.len())),
            IntersectionInner::Answer(None) => (0, Some(0)),
            IntersectionInner::Answer(Some(_)) => (1, Some(1)),
        }
    }

    fn min(mut self) -> Option<&'a T> {
        self.next()
    }
}

impl<T: Ord, A: Allocator> FusedIterator for Intersection<'_, T, A> {}

impl<T> Clone for Union<'_, T> {
    fn clone(&self) -> Self {
        Union(self.0.clone())
    }
}
impl<'a, T: Ord> Iterator for Union<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        let (a_next, b_next) = self.0.nexts(Self::Item::cmp);
        a_next.or(b_next)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (a_len, b_len) = self.0.lens();
        // No checked_add - see SymmetricDifference::size_hint.
        (max(a_len, b_len), Some(a_len + b_len))
    }

    fn min(mut self) -> Option<&'a T> {
        self.next()
    }
}

impl<T: Ord> FusedIterator for Union<'_, T> {}

impl<T, A: Allocator> TryFromIteratorIn<T, A> for BTreeSet<T, A>
where
    T: Ord,
{
    #[inline]
    fn try_from_iter_in<I>(iter: I, alloc: A) -> Result<Self, Error>
    where
        I: IntoIterator<Item = T>,
    {
        let mut this = BTreeSet::new_in(alloc);

        for value in iter {
            this.try_insert(value)?;
        }

        Ok(this)
    }
}

#[cfg(test)]
impl<T> FromIterator<T> for BTreeSet<T>
where
    T: Ord,
{
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        Self::try_from_iter_in(iter, Global).abort()
    }
}

impl<T, const N: usize> TryFrom<[T; N]> for BTreeSet<T>
where
    T: Ord,
{
    type Error = Error;

    #[inline]
    fn try_from(values: [T; N]) -> Result<Self, Self::Error> {
        let mut this = BTreeSet::new();

        for value in values {
            this.try_insert(value)?;
        }

        Ok(this)
    }
}

#[cfg(test)]
mod tests;
