//! An ordered map based on a B-Tree.

use core::borrow::Borrow;
use core::cmp::Ordering;
use core::convert::Infallible;
use core::fmt::{self, Debug};
use core::hash::{Hash, Hasher};
use core::iter::FusedIterator;
use core::marker::PhantomData;
use core::mem::{self, ManuallyDrop};
use core::ops::{Bound, Index, RangeBounds};

use crate::ptr;
#[cfg(test)]
use crate::testing::*;

use crate::alloc::{AllocError, Allocator, Global};
use crate::boxed::Box;
use crate::clone::TryClone;
use crate::error::{CustomError, Error};
use crate::iter::{TryExtend, TryFromIteratorIn};

use super::borrow::DormantMutRef;
use super::navigate::{LazyLeafRange, LeafRange};
use super::node::{self, marker, ForceResult::*, Handle, NodeRef, Root};
use super::search::{SearchBound, SearchResult::*};
use super::set_val::SetValZST;
use super::Recover;

pub use entry::{Entry, OccupiedEntry, OccupiedError, VacantEntry};
mod entry;

pub(crate) type CmpFn<C, Q, E> = fn(&mut C, &Q, &Q) -> Result<Ordering, E>;

use Entry::*;

macro_rules! into_iter {
    ($this:expr) => {{
        let length = mem::take(&mut $this.length);

        if let Some(root) = $this.root.take() {
            let full_range = root.into_dying().full_range();

            IntoIter {
                range: full_range,
                length,
                alloc: &*$this.alloc,
            }
        } else {
            IntoIter {
                range: LazyLeafRange::none(),
                length: 0,
                alloc: &*$this.alloc,
            }
        }
    }};
}

#[inline(always)]
pub(crate) fn into_ok<T>(result: Result<T, Infallible>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => match error {},
    }
}

#[inline(always)]
pub(crate) fn infallible_cmp<T: ?Sized>(_: &mut (), a: &T, b: &T) -> Result<Ordering, Infallible>
where
    T: Ord,
{
    Ok(a.cmp(b))
}

/// Minimum number of elements in a node that is not a root.
/// We might temporarily have fewer elements during methods.
pub(super) const MIN_LEN: usize = node::MIN_LEN_AFTER_SPLIT;

// A tree in a `BTreeMap` is a tree in the `node` module with additional invariants:
// - Keys must appear in ascending order (according to the key's type).
// - Every non-leaf node contains at least 1 element (has at least 2 children).
// - Every non-root node contains at least MIN_LEN elements.
//
// An empty map is represented either by the absence of a root node or by a
// root node that is an empty leaf.

/// An ordered map based on a [B-Tree].
///
/// B-Trees represent a fundamental compromise between cache-efficiency and actually minimizing
/// the amount of work performed in a search. In theory, a binary search tree (BST) is the optimal
/// choice for a sorted map, as a perfectly balanced BST performs the theoretical minimum amount of
/// comparisons necessary to find an element (log<sub>2</sub>n). However, in practice the way this
/// is done is *very* inefficient for modern computer architectures. In particular, every element
/// is stored in its own individually heap-allocated node. This means that every single insertion
/// triggers a heap-allocation, and every single comparison should be a cache-miss. Since these
/// are both notably expensive things to do in practice, we are forced to, at the very least,
/// reconsider the BST strategy.
///
/// A B-Tree instead makes each node contain B-1 to 2B-1 elements in a contiguous array. By doing
/// this, we reduce the number of allocations by a factor of B, and improve cache efficiency in
/// searches. However, this does mean that searches will have to do *more* comparisons on average.
/// The precise number of comparisons depends on the node search strategy used. For optimal cache
/// efficiency, one could search the nodes linearly. For optimal comparisons, one could search
/// the node using binary search. As a compromise, one could also perform a linear search
/// that initially only checks every i<sup>th</sup> element for some choice of i.
///
/// Currently, our implementation simply performs naive linear search. This provides excellent
/// performance on *small* nodes of elements which are cheap to compare. However in the future we
/// would like to further explore choosing the optimal search strategy based on the choice of B,
/// and possibly other factors. Using linear search, searching for a random element is expected
/// to take B * log(n) comparisons, which is generally worse than a BST. In practice,
/// however, performance is excellent.
///
/// It is a logic error for a key to be modified in such a way that the key's ordering relative to
/// any other key, as determined by the [`Ord`] trait, changes while it is in the map. This is
/// normally only possible through [`Cell`], [`RefCell`], global state, I/O, or unsafe code.
/// The behavior resulting from such a logic error is not specified, but will be encapsulated to the
/// `BTreeMap` that observed the logic error and not result in undefined behavior. This could
/// include panics, incorrect results, aborts, memory leaks, and non-termination.
///
/// Iterators obtained from functions such as [`BTreeMap::iter`], [`BTreeMap::values`], or
/// [`BTreeMap::keys`] produce their items in order by key, and take worst-case logarithmic and
/// amortized constant time per item returned.
///
/// [B-Tree]: https://en.wikipedia.org/wiki/B-tree
/// [`Cell`]: core::cell::Cell
/// [`RefCell`]: core::cell::RefCell
///
/// # Examples
///
/// ```
/// use rune::alloc::BTreeMap;
///
/// // type inference lets us omit an explicit type signature (which
/// // would be `BTreeMap<&str, &str>` in this example).
/// let mut movie_reviews = BTreeMap::new();
///
/// // review some movies.
/// movie_reviews.try_insert("Office Space", "Deals with real issues in the workplace.")?;
/// movie_reviews.try_insert("Pulp Fiction", "Masterpiece.")?;
/// movie_reviews.try_insert("The Godfather", "Very enjoyable.")?;
/// movie_reviews.try_insert("The Blues Brothers", "Eye lyked it a lot.")?;
///
/// // check for a specific one.
/// if !movie_reviews.contains_key("Les Misérables") {
///     println!("We've got {} reviews, but Les Misérables ain't one.",
///              movie_reviews.len());
/// }
///
/// // oops, this review has a lot of spelling mistakes, let's delete it.
/// movie_reviews.remove("The Blues Brothers");
///
/// // look up the values associated with some keys.
/// let to_find = ["Up!", "Office Space"];
/// for movie in &to_find {
///     match movie_reviews.get(movie) {
///        Some(review) => println!("{movie}: {review}"),
///        None => println!("{movie} is unreviewed.")
///     }
/// }
///
/// // Look up the value for a key (will panic if the key is not found).
/// println!("Movie review: {}", movie_reviews["Office Space"]);
///
/// // iterate over everything.
/// for (movie, review) in &movie_reviews {
///     println!("{movie}: \"{review}\"");
/// }
/// # Ok::<_, rune::alloc::Error>(())
/// ```
///
/// A `BTreeMap` with a known list of items can be initialized from an array:
///
/// ```
/// use rune::alloc::BTreeMap;
///
/// let solar_distance = BTreeMap::try_from([
///     ("Mercury", 0.4),
///     ("Venus", 0.7),
///     ("Earth", 1.0),
///     ("Mars", 1.5),
/// ])?;
/// # Ok::<_, rune::alloc::Error>(())
/// ```
///
/// `BTreeMap` implements an [`Entry API`], which allows for complex
/// methods of getting, setting, updating and removing keys and their values:
///
/// [`Entry API`]: BTreeMap::entry
///
/// ```
/// use rune::alloc::BTreeMap;
///
/// // type inference lets us omit an explicit type signature (which
/// // would be `BTreeMap<&str, u8>` in this example).
/// let mut player_stats = BTreeMap::new();
///
/// fn random_stat_buff() -> u8 {
///     // could actually return some random value here - let's just return
///     // some fixed value for now
///     42
/// }
///
/// // insert a key only if it doesn't already exist
/// player_stats.entry("health").or_try_insert(100)?;
///
/// // insert a key using a function that provides a new value only if it
/// // doesn't already exist
/// player_stats.entry("defence").or_try_insert_with(random_stat_buff)?;
///
/// // update a key, guarding against the key possibly not being set
/// let stat = player_stats.entry("attack").or_try_insert(100)?;
/// *stat += random_stat_buff();
///
/// // modify an entry before an insert with in-place mutation
/// player_stats.entry("mana").and_modify(|mana| *mana += 200).or_try_insert(100)?;
/// # Ok::<_, rune::alloc::Error>(())
/// ```
pub struct BTreeMap<K, V, A: Allocator = Global> {
    root: Option<Root<K, V>>,
    length: usize,
    /// `ManuallyDrop` to control drop order (needs to be dropped after all the nodes).
    pub(super) alloc: ManuallyDrop<A>,
    // For dropck; the `Box` avoids making the `Unpin` impl more strict than before
    _marker: PhantomData<Box<(K, V)>>,
}

#[cfg(rune_nightly)]
unsafe impl<#[may_dangle] K, #[may_dangle] V, A: Allocator> Drop for BTreeMap<K, V, A> {
    fn drop(&mut self) {
        drop(unsafe { ptr::read(self) }.into_iter())
    }
}

#[cfg(not(rune_nightly))]
impl<K, V, A: Allocator> Drop for BTreeMap<K, V, A> {
    fn drop(&mut self) {
        drop(unsafe { ptr::read(self) }.into_iter())
    }
}

// FIXME: This implementation is "wrong", but changing it would be a breaking change.
// (The bounds of the automatic `UnwindSafe` implementation have been like this since Rust 1.50.)
// Maybe we can fix it nonetheless with a crater run, or if the `UnwindSafe`
// traits are deprecated, or disarmed (no longer causing hard errors) in the future.
impl<K, V, A: Allocator> core::panic::UnwindSafe for BTreeMap<K, V, A>
where
    A: core::panic::UnwindSafe,
    K: core::panic::RefUnwindSafe,
    V: core::panic::RefUnwindSafe,
{
}

impl<K: TryClone, V: TryClone, A: Allocator + Clone> TryClone for BTreeMap<K, V, A> {
    fn try_clone(&self) -> Result<BTreeMap<K, V, A>, Error> {
        fn clone_subtree<'a, K: TryClone, V: TryClone, A: Allocator + Clone>(
            node: NodeRef<marker::Immut<'a>, K, V, marker::LeafOrInternal>,
            alloc: &A,
        ) -> Result<BTreeMap<K, V, A>, Error>
        where
            K: 'a,
            V: 'a,
        {
            match node.force() {
                Leaf(leaf) => {
                    let mut out_tree = BTreeMap {
                        root: Some(Root::new(alloc)?),
                        length: 0,
                        alloc: ManuallyDrop::new(alloc.clone()),
                        _marker: PhantomData,
                    };

                    {
                        let root = out_tree.root.as_mut().unwrap(); // unwrap succeeds because we just wrapped
                        let mut out_node = match root.borrow_mut().force() {
                            Leaf(leaf) => leaf,
                            Internal(_) => unreachable!(),
                        };

                        let mut in_edge = leaf.first_edge();
                        while let Ok(kv) = in_edge.right_kv() {
                            let (k, v) = kv.into_kv();
                            in_edge = kv.right_edge();

                            out_node.push(k.try_clone()?, v.try_clone()?);
                            out_tree.length += 1;
                        }
                    }

                    Ok(out_tree)
                }
                Internal(internal) => {
                    let mut out_tree = clone_subtree(internal.first_edge().descend(), alloc)?;

                    {
                        let out_root = out_tree.root.as_mut().unwrap();
                        let mut out_node = out_root.push_internal_level(alloc)?;
                        let mut in_edge = internal.first_edge();
                        while let Ok(kv) = in_edge.right_kv() {
                            let (k, v) = kv.into_kv();
                            in_edge = kv.right_edge();

                            let k = (*k).try_clone()?;
                            let v = (*v).try_clone()?;
                            let subtree = clone_subtree(in_edge.descend(), alloc)?;

                            // We can't destructure subtree directly
                            // because BTreeMap implements Drop
                            let (subroot, sublength) = unsafe {
                                let subtree = ManuallyDrop::new(subtree);
                                let root = ptr::read(&subtree.root);
                                let length = subtree.length;
                                (root, length)
                            };

                            let subroot = match subroot {
                                Some(subroot) => subroot,
                                None => Root::new(alloc)?,
                            };

                            out_node.push(k, v, subroot);
                            out_tree.length += 1 + sublength;
                        }
                    }

                    Ok(out_tree)
                }
            }
        }

        if self.is_empty() {
            Ok(BTreeMap::new_in((*self.alloc).clone()))
        } else {
            clone_subtree(self.root.as_ref().unwrap().reborrow(), &*self.alloc) // unwrap succeeds because not empty
        }
    }
}

#[cfg(test)]
impl<K: TryClone, V: TryClone, A: Allocator + Clone> Clone for BTreeMap<K, V, A> {
    #[inline]
    fn clone(&self) -> Self {
        self.try_clone().abort()
    }

    #[inline]
    fn clone_from(&mut self, source: &Self) {
        self.try_clone_from(source).abort()
    }
}

impl<K, Q: ?Sized, A: Allocator> Recover<Q> for BTreeMap<K, SetValZST, A>
where
    K: Borrow<Q>,
{
    type Key = K;

    fn get<C: ?Sized, E>(&self, cx: &mut C, key: &Q, cmp: CmpFn<C, Q, E>) -> Result<Option<&K>, E> {
        let Some(root_node) = self.root.as_ref() else {
            return Ok(None);
        };

        let root_node = root_node.reborrow();

        Ok(match root_node.search_tree(cx, key, cmp)? {
            Found(handle) => Some(handle.into_kv().0),
            GoDown(_) => None,
        })
    }

    fn take<C: ?Sized, E>(
        &mut self,
        cx: &mut C,
        key: &Q,
        cmp: CmpFn<C, Q, E>,
    ) -> Result<Option<K>, E> {
        let (map, dormant_map) = DormantMutRef::new(self);

        let Some(root_node) = map.root.as_mut() else {
            return Ok(None);
        };

        let root_node = root_node.borrow_mut();

        Ok(match root_node.search_tree(cx, key, cmp)? {
            Found(handle) => {
                let entry = OccupiedEntry {
                    handle,
                    dormant_map,
                    alloc: &*map.alloc,
                    _marker: PhantomData,
                };

                Some(entry.remove_kv().0)
            }
            GoDown(_) => None,
        })
    }

    fn try_replace<C: ?Sized, E>(
        &mut self,
        cx: &mut C,
        key: K,
        cmp: CmpFn<C, Q, E>,
    ) -> Result<Result<Option<K>, AllocError>, E> {
        let (map, dormant_map) = DormantMutRef::new(self);

        let root_node = match &mut map.root {
            Some(root) => root,
            None => {
                let root = match Root::new(&*map.alloc) {
                    Ok(root) => root,
                    Err(error) => return Ok(Err(error)),
                };

                map.root.insert(root)
            }
        };

        let root_node = root_node.borrow_mut();

        match root_node.search_tree(cx, key.borrow(), cmp)? {
            Found(mut kv) => Ok(Ok(Some(mem::replace(kv.key_mut(), key)))),
            GoDown(handle) => {
                let entry = VacantEntry {
                    key,
                    handle: Some(handle),
                    dormant_map,
                    alloc: &*map.alloc,
                    _marker: PhantomData,
                };

                if let Err(error) = entry.try_insert(SetValZST) {
                    return Ok(Err(error));
                }

                Ok(Ok(None))
            }
        }
    }
}

/// A raw iterator over a map where the caller is responsible for ensuring that
/// it doesn't outlive the data it's iterating over.
///
/// See [BTreeMap::iter_raw].
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct IterRaw<K, V> {
    range: LazyLeafRange<marker::Raw, K, V>,
    length: usize,
}

impl<K, V> Iterator for IterRaw<K, V> {
    type Item = (*const K, *const V);

    fn next(&mut self) -> Option<(*const K, *const V)> {
        if self.length == 0 {
            None
        } else {
            self.length -= 1;
            Some(unsafe { self.range.next_unchecked() })
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.length, Some(self.length))
    }

    fn last(mut self) -> Option<(*const K, *const V)> {
        self.next_back()
    }
}

impl<K, V> FusedIterator for IterRaw<K, V> {}

impl<K, V> DoubleEndedIterator for IterRaw<K, V> {
    fn next_back(&mut self) -> Option<(*const K, *const V)> {
        if self.length == 0 {
            None
        } else {
            self.length -= 1;
            Some(unsafe { self.range.next_back_unchecked() })
        }
    }
}

impl<K, V> ExactSizeIterator for IterRaw<K, V> {
    fn len(&self) -> usize {
        self.length
    }
}

impl<K, V> Clone for IterRaw<K, V> {
    fn clone(&self) -> Self {
        IterRaw {
            range: self.range.clone(),
            length: self.length,
        }
    }
}

/// An iterator over the entries of a `BTreeMap`.
///
/// This `struct` is created by the [`iter`] method on [`BTreeMap`]. See its
/// documentation for more.
///
/// [`iter`]: BTreeMap::iter
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct Iter<'a, K: 'a, V: 'a> {
    range: LazyLeafRange<marker::Immut<'a>, K, V>,
    length: usize,
}

impl<K: fmt::Debug, V: fmt::Debug> fmt::Debug for Iter<'_, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

impl<'a, K: 'a, V: 'a> Default for Iter<'a, K, V> {
    /// Creates an empty `btree_map::Iter`.
    ///
    /// ```
    /// use rune::alloc::btree_map;
    ///
    /// let iter: btree_map::Iter<'_, u8, u8> = Default::default();
    /// assert_eq!(iter.len(), 0);
    /// ```
    fn default() -> Self {
        Iter {
            range: Default::default(),
            length: 0,
        }
    }
}

/// A mutable iterator over the entries of a `BTreeMap`.
///
/// This `struct` is created by the [`iter_mut`] method on [`BTreeMap`]. See its
/// documentation for more.
///
/// [`iter_mut`]: BTreeMap::iter_mut
pub struct IterMut<'a, K: 'a, V: 'a> {
    range: LazyLeafRange<marker::ValMut<'a>, K, V>,
    length: usize,

    // Be invariant in `K` and `V`
    _marker: PhantomData<&'a mut (K, V)>,
}

impl<K: fmt::Debug, V: fmt::Debug> fmt::Debug for IterMut<'_, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let range = Iter {
            range: self.range.reborrow(),
            length: self.length,
        };
        f.debug_list().entries(range).finish()
    }
}

impl<'a, K: 'a, V: 'a> Default for IterMut<'a, K, V> {
    /// Creates an empty `btree_map::IterMut`.
    ///
    /// ```
    /// use rune::alloc::btree_map;
    ///
    /// let iter: btree_map::IterMut<'_, u8, u8> = Default::default();
    /// assert_eq!(iter.len(), 0);
    /// ```
    fn default() -> Self {
        IterMut {
            range: Default::default(),
            length: 0,
            _marker: PhantomData {},
        }
    }
}

/// An owning iterator over the entries of a `BTreeMap`.
///
/// This `struct` is created by the [`into_iter`] method on [`BTreeMap`]
/// (provided by the [`IntoIterator`] trait). See its documentation for more.
///
/// [`into_iter`]: IntoIterator::into_iter
pub struct IntoIter<K, V, A: Allocator = Global> {
    range: LazyLeafRange<marker::Dying, K, V>,
    length: usize,
    /// The BTreeMap will outlive this IntoIter so we don't care about drop order for `alloc`.
    alloc: A,
}

impl<K, V, A: Allocator> IntoIter<K, V, A> {
    /// Returns an iterator of references over the remaining items.
    #[inline]
    pub(super) fn iter(&self) -> Iter<'_, K, V> {
        Iter {
            range: self.range.reborrow(),
            length: self.length,
        }
    }
}

impl<K: Debug, V: Debug, A: Allocator> Debug for IntoIter<K, V, A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<K, V, A> Default for IntoIter<K, V, A>
where
    A: Allocator + Default,
{
    /// Creates an empty `btree_map::IntoIter`.
    ///
    /// ```
    /// use rune::alloc::btree_map;
    ///
    /// let iter: btree_map::IntoIter<u8, u8> = Default::default();
    /// assert_eq!(iter.len(), 0);
    /// ```
    fn default() -> Self {
        IntoIter {
            range: Default::default(),
            length: 0,
            alloc: Default::default(),
        }
    }
}

/// An iterator over the keys of a `BTreeMap`.
///
/// This `struct` is created by the [`keys`] method on [`BTreeMap`]. See its
/// documentation for more.
///
/// [`keys`]: BTreeMap::keys
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct Keys<'a, K, V> {
    inner: Iter<'a, K, V>,
}

impl<K: fmt::Debug, V> fmt::Debug for Keys<'_, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

/// An iterator over the values of a `BTreeMap`.
///
/// This `struct` is created by the [`values`] method on [`BTreeMap`]. See its
/// documentation for more.
///
/// [`values`]: BTreeMap::values
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct Values<'a, K, V> {
    inner: Iter<'a, K, V>,
}

impl<K, V: fmt::Debug> fmt::Debug for Values<'_, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

/// A mutable iterator over the values of a `BTreeMap`.
///
/// This `struct` is created by the [`values_mut`] method on [`BTreeMap`]. See its
/// documentation for more.
///
/// [`values_mut`]: BTreeMap::values_mut
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct ValuesMut<'a, K, V> {
    inner: IterMut<'a, K, V>,
}

impl<K, V: fmt::Debug> fmt::Debug for ValuesMut<'_, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list()
            .entries(self.inner.iter().map(|(_, val)| val))
            .finish()
    }
}

/// An owning iterator over the keys of a `BTreeMap`.
///
/// This `struct` is created by the [`into_keys`] method on [`BTreeMap`]. See
/// its documentation for more.
///
/// [`into_keys`]: BTreeMap::into_keys
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct IntoKeys<K, V, A: Allocator = Global> {
    inner: IntoIter<K, V, A>,
}

impl<K: fmt::Debug, V, A: Allocator> fmt::Debug for IntoKeys<K, V, A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list()
            .entries(self.inner.iter().map(|(key, _)| key))
            .finish()
    }
}

/// An owning iterator over the values of a `BTreeMap`.
///
/// This `struct` is created by the [`into_values`] method on [`BTreeMap`]. See
/// its documentation for more.
///
/// [`into_values`]: BTreeMap::into_values
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct IntoValues<K, V, A: Allocator = Global> {
    inner: IntoIter<K, V, A>,
}

impl<K, V: fmt::Debug, A: Allocator> fmt::Debug for IntoValues<K, V, A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list()
            .entries(self.inner.iter().map(|(_, val)| val))
            .finish()
    }
}

/// An iterator over a sub-range of entries in a `BTreeMap`.
///
/// This `struct` is created by the [`range`] method on [`BTreeMap`]. See its
/// documentation for more.
///
/// [`range`]: BTreeMap::range
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct Range<'a, K: 'a, V: 'a> {
    inner: LeafRange<marker::Immut<'a>, K, V>,
}

impl<K: fmt::Debug, V: fmt::Debug> fmt::Debug for Range<'_, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

/// A mutable iterator over a sub-range of entries in a `BTreeMap`.
///
/// This `struct` is created by the [`range_mut`] method on [`BTreeMap`]. See its
/// documentation for more.
///
/// [`range_mut`]: BTreeMap::range_mut
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct RangeMut<'a, K: 'a, V: 'a> {
    inner: LeafRange<marker::ValMut<'a>, K, V>,

    // Be invariant in `K` and `V`
    _marker: PhantomData<&'a mut (K, V)>,
}

impl<K: fmt::Debug, V: fmt::Debug> fmt::Debug for RangeMut<'_, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let range = Range {
            inner: self.inner.reborrow(),
        };
        f.debug_list().entries(range).finish()
    }
}

impl<K, V> BTreeMap<K, V> {
    /// Makes a new, empty `BTreeMap`.
    ///
    /// Does not allocate anything on its own.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use rune::alloc::BTreeMap;
    ///
    /// let mut map = BTreeMap::new();
    ///
    /// // entries can now be inserted into the empty map
    /// map.try_insert(1, "a")?;
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[must_use]
    pub const fn new() -> BTreeMap<K, V> {
        BTreeMap {
            root: None,
            length: 0,
            alloc: ManuallyDrop::new(Global),
            _marker: PhantomData,
        }
    }

    #[cfg(test)]
    pub(crate) fn from<const N: usize>(value: [(K, V); N]) -> Self
    where
        K: Ord,
    {
        Self::try_from(value).abort()
    }
}

impl<K, V, A: Allocator> BTreeMap<K, V, A> {
    /// Makes a new empty BTreeMap with a reasonable choice for B.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use rune::alloc::BTreeMap;
    /// use rune::alloc::alloc::Global;
    ///
    /// let mut map = BTreeMap::new_in(Global);
    ///
    /// // entries can now be inserted into the empty map
    /// map.try_insert(1, "a")?;
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn new_in(alloc: A) -> BTreeMap<K, V, A> {
        BTreeMap {
            root: None,
            length: 0,
            alloc: ManuallyDrop::new(alloc),
            _marker: PhantomData,
        }
    }
}

impl<K, V, A: Allocator> BTreeMap<K, V, A> {
    /// Clears the map, removing all elements.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use rune::alloc::BTreeMap;
    ///
    /// let mut a = BTreeMap::new();
    /// a.try_insert(1, "a")?;
    /// a.clear();
    /// assert!(a.is_empty());
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn clear(&mut self) {
        drop(into_iter!(self));
    }
}

impl<K, V, A: Allocator> BTreeMap<K, V, A> {
    /// Returns a reference to the value corresponding to the key.
    ///
    /// The key may be any borrowed form of the map's key type, but the ordering
    /// on the borrowed form *must* match the ordering on the key type.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use rune::alloc::BTreeMap;
    ///
    /// let mut map = BTreeMap::new();
    /// map.try_insert(1, "a")?;
    /// assert_eq!(map.get(&1), Some(&"a"));
    /// assert_eq!(map.get(&2), None);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn get<Q: ?Sized>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q> + Ord,
        Q: Ord,
    {
        into_ok(self.get_with(&mut (), key, infallible_cmp))
    }

    pub(crate) fn get_with<C: ?Sized, Q: ?Sized, E>(
        &self,
        cx: &mut C,
        key: &Q,
        cmp: CmpFn<C, Q, E>,
    ) -> Result<Option<&V>, E>
    where
        K: Borrow<Q>,
    {
        let Some(root_node) = self.root.as_ref().map(NodeRef::reborrow) else {
            return Ok(None);
        };

        Ok(match root_node.search_tree(cx, key, cmp)? {
            Found(handle) => Some(handle.into_kv().1),
            GoDown(_) => None,
        })
    }

    /// Returns the key-value pair corresponding to the supplied key.
    ///
    /// The supplied key may be any borrowed form of the map's key type, but the ordering
    /// on the borrowed form *must* match the ordering on the key type.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::BTreeMap;
    ///
    /// let mut map = BTreeMap::new();
    /// map.try_insert(1, "a")?;
    /// assert_eq!(map.get_key_value(&1), Some((&1, &"a")));
    /// assert_eq!(map.get_key_value(&2), None);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn get_key_value<Q: ?Sized>(&self, k: &Q) -> Option<(&K, &V)>
    where
        K: Borrow<Q> + Ord,
        Q: Ord,
    {
        let root_node = self.root.as_ref()?.reborrow();
        match into_ok(root_node.search_tree(&mut (), k, infallible_cmp)) {
            Found(handle) => Some(handle.into_kv()),
            GoDown(_) => None,
        }
    }

    /// Returns the first key-value pair in the map.
    /// The key in this pair is the minimum key in the map.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use rune::alloc::BTreeMap;
    ///
    /// let mut map = BTreeMap::new();
    /// assert_eq!(map.first_key_value(), None);
    /// map.try_insert(1, "b")?;
    /// map.try_insert(2, "a")?;
    /// assert_eq!(map.first_key_value(), Some((&1, &"b")));
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn first_key_value(&self) -> Option<(&K, &V)> {
        let root_node = self.root.as_ref()?.reborrow();
        root_node
            .first_leaf_edge()
            .right_kv()
            .ok()
            .map(Handle::into_kv)
    }

    /// Returns the first entry in the map for in-place manipulation.
    /// The key of this entry is the minimum key in the map.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::BTreeMap;
    ///
    /// let mut map = BTreeMap::new();
    /// map.try_insert(1, "a")?;
    /// map.try_insert(2, "b")?;
    ///
    /// if let Some(mut entry) = map.first_entry() {
    ///     if *entry.key() > 0 {
    ///         entry.insert("first");
    ///     }
    /// }
    ///
    /// assert_eq!(*map.get(&1).unwrap(), "first");
    /// assert_eq!(*map.get(&2).unwrap(), "b");
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn first_entry(&mut self) -> Option<OccupiedEntry<'_, K, V, A>> {
        let (map, dormant_map) = DormantMutRef::new(self);
        let root_node = map.root.as_mut()?.borrow_mut();
        let kv = root_node.first_leaf_edge().right_kv().ok()?;
        Some(OccupiedEntry {
            handle: kv.forget_node_type(),
            dormant_map,
            alloc: &*map.alloc,
            _marker: PhantomData,
        })
    }

    /// Removes and returns the first element in the map.
    /// The key of this element is the minimum key that was in the map.
    ///
    /// # Examples
    ///
    /// Draining elements in ascending order, while keeping a usable map each iteration.
    ///
    /// ```
    /// use rune::alloc::BTreeMap;
    ///
    /// let mut map = BTreeMap::new();
    /// map.try_insert(1, "a")?;
    /// map.try_insert(2, "b")?;
    /// while let Some((key, _val)) = map.pop_first() {
    ///     assert!(map.iter().all(|(k, _v)| *k > key));
    /// }
    /// assert!(map.is_empty());
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn pop_first(&mut self) -> Option<(K, V)> {
        self.first_entry().map(|entry| entry.remove_entry())
    }

    /// Returns the last key-value pair in the map.
    /// The key in this pair is the maximum key in the map.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use rune::alloc::BTreeMap;
    ///
    /// let mut map = BTreeMap::new();
    /// map.try_insert(1, "b")?;
    /// map.try_insert(2, "a")?;
    /// assert_eq!(map.last_key_value(), Some((&2, &"a")));
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn last_key_value(&self) -> Option<(&K, &V)> {
        let root_node = self.root.as_ref()?.reborrow();
        root_node
            .last_leaf_edge()
            .left_kv()
            .ok()
            .map(Handle::into_kv)
    }

    /// Returns the last entry in the map for in-place manipulation.
    /// The key of this entry is the maximum key in the map.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::BTreeMap;
    ///
    /// let mut map = BTreeMap::new();
    /// map.try_insert(1, "a")?;
    /// map.try_insert(2, "b")?;
    ///
    /// if let Some(mut entry) = map.last_entry() {
    ///     if *entry.key() > 0 {
    ///         entry.insert("last");
    ///     }
    /// }
    ///
    /// assert_eq!(*map.get(&1).unwrap(), "a");
    /// assert_eq!(*map.get(&2).unwrap(), "last");
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn last_entry(&mut self) -> Option<OccupiedEntry<'_, K, V, A>> {
        let (map, dormant_map) = DormantMutRef::new(self);
        let root_node = map.root.as_mut()?.borrow_mut();
        let kv = root_node.last_leaf_edge().left_kv().ok()?;
        Some(OccupiedEntry {
            handle: kv.forget_node_type(),
            dormant_map,
            alloc: &*map.alloc,
            _marker: PhantomData,
        })
    }

    /// Removes and returns the last element in the map.
    /// The key of this element is the maximum key that was in the map.
    ///
    /// # Examples
    ///
    /// Draining elements in descending order, while keeping a usable map each iteration.
    ///
    /// ```
    /// use rune::alloc::BTreeMap;
    ///
    /// let mut map = BTreeMap::new();
    /// map.try_insert(1, "a")?;
    /// map.try_insert(2, "b")?;
    ///
    /// while let Some((key, _val)) = map.pop_last() {
    ///     assert!(map.iter().all(|(k, _v)| *k < key));
    /// }
    ///
    /// assert!(map.is_empty());
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn pop_last(&mut self) -> Option<(K, V)> {
        self.last_entry().map(|entry| entry.remove_entry())
    }

    /// Returns `true` if the map contains a value for the specified key.
    ///
    /// The key may be any borrowed form of the map's key type, but the ordering
    /// on the borrowed form *must* match the ordering on the key type.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use rune::alloc::BTreeMap;
    ///
    /// let mut map = BTreeMap::new();
    /// map.try_insert(1, "a")?;
    ///
    /// assert_eq!(map.contains_key(&1), true);
    /// assert_eq!(map.contains_key(&2), false);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn contains_key<Q: ?Sized>(&self, key: &Q) -> bool
    where
        K: Borrow<Q> + Ord,
        Q: Ord,
    {
        into_ok(self.contains_key_with(&mut (), key, infallible_cmp))
    }

    pub(crate) fn contains_key_with<C: ?Sized, Q: ?Sized, E>(
        &self,
        cx: &mut C,
        key: &Q,
        cmp: CmpFn<C, Q, E>,
    ) -> Result<bool, E>
    where
        K: Borrow<Q> + Ord,
        Q: Ord,
    {
        Ok(self.get_with(cx, key, cmp)?.is_some())
    }

    /// Returns a mutable reference to the value corresponding to the key.
    ///
    /// The key may be any borrowed form of the map's key type, but the ordering
    /// on the borrowed form *must* match the ordering on the key type.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use rune::alloc::BTreeMap;
    ///
    /// let mut map = BTreeMap::new();
    ///
    /// map.try_insert(1, "a")?;
    ///
    /// if let Some(x) = map.get_mut(&1) {
    ///     *x = "b";
    /// }
    ///
    /// assert_eq!(map[&1], "b");
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    // See `get` for implementation notes, this is basically a copy-paste with mut's added
    pub fn get_mut<Q: ?Sized>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q> + Ord,
        Q: Ord,
    {
        into_ok(self.get_mut_with(&mut (), key, infallible_cmp))
    }

    /// Like [`BTreeMap::get_mut`] but allows for custom value comparisons.
    ///
    /// The comparison implementation should to be coherent with the ones used
    /// for insertion, else unexpected values might be accessed.
    pub fn get_mut_with<C: ?Sized, Q: ?Sized, E>(
        &mut self,
        cx: &mut C,
        key: &Q,
        cmp: CmpFn<C, Q, E>,
    ) -> Result<Option<&mut V>, E>
    where
        K: Borrow<Q>,
    {
        let Some(root_node) = self.root.as_mut().map(NodeRef::borrow_mut) else {
            return Ok(None);
        };

        Ok(match root_node.search_tree(cx, key, cmp)? {
            Found(handle) => Some(handle.into_val_mut()),
            GoDown(_) => None,
        })
    }

    /// Inserts a key-value pair into the map.
    ///
    /// If the map did not have this key present, `None` is returned.
    ///
    /// If the map did have this key present, the value is updated, and the old
    /// value is returned. The key is not updated, though; this matters for
    /// types that can be `==` without being identical. See the [module-level
    /// documentation] for more.
    ///
    /// [module-level documentation]: index.html#insert-and-complex-keys
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use rune::alloc::BTreeMap;
    ///
    /// let mut map = BTreeMap::new();
    /// assert_eq!(map.try_insert(37, "a")?, None);
    /// assert_eq!(map.is_empty(), false);
    ///
    /// map.try_insert(37, "b")?;
    /// assert_eq!(map.try_insert(37, "c")?, Some("b"));
    /// assert_eq!(map[&37], "c");
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn try_insert(&mut self, key: K, value: V) -> Result<Option<V>, AllocError>
    where
        K: Ord,
    {
        match self.entry(key) {
            Occupied(mut entry) => Ok(Some(entry.insert(value))),
            Vacant(entry) => {
                entry.try_insert(value)?;
                Ok(None)
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn insert(&mut self, key: K, value: V) -> Option<V>
    where
        K: Ord,
    {
        self.try_insert(key, value).abort()
    }

    /// Tries to insert a key-value pair into the map, and returns a mutable
    /// reference to the value in the entry.
    ///
    /// If the map already had this key present, nothing is updated, and an
    /// error containing the occupied entry and the value is returned.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use rune::alloc::BTreeMap;
    /// use rune::alloc::error::CustomError;
    ///
    /// let mut map = BTreeMap::new();
    /// assert_eq!(map.try_insert_or(37, "a").unwrap(), &"a");
    ///
    /// if let CustomError::Custom(err) = map.try_insert_or(37, "b").unwrap_err() {
    ///     assert_eq!(err.entry.key(), &37);
    ///     assert_eq!(err.entry.get(), &"a");
    ///     assert_eq!(err.value, "b");
    /// }
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn try_insert_or(
        &mut self,
        key: K,
        value: V,
    ) -> Result<&mut V, CustomError<OccupiedError<'_, K, V, A>>>
    where
        K: Ord,
    {
        match self.entry(key) {
            Occupied(entry) => Err(CustomError::Custom(OccupiedError { entry, value })),
            Vacant(entry) => Ok(entry.try_insert(value)?),
        }
    }

    #[cfg(test)]
    pub(crate) fn insert_or(
        &mut self,
        key: K,
        value: V,
    ) -> Result<&mut V, OccupiedError<'_, K, V, A>>
    where
        K: Ord,
    {
        self.try_insert_or(key, value).custom_result()
    }

    /// Removes a key from the map, returning the value at the key if the key
    /// was previously in the map.
    ///
    /// The key may be any borrowed form of the map's key type, but the ordering
    /// on the borrowed form *must* match the ordering on the key type.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use rune::alloc::BTreeMap;
    ///
    /// let mut map = BTreeMap::new();
    /// map.try_insert(1, "a")?;
    /// assert_eq!(map.remove(&1), Some("a"));
    /// assert_eq!(map.remove(&1), None);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn remove<Q: ?Sized>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q> + Ord,
        Q: Ord,
    {
        self.remove_entry(key).map(|(_, v)| v)
    }

    /// Removes a key from the map, returning the stored key and value if the key
    /// was previously in the map.
    ///
    /// The key may be any borrowed form of the map's key type, but the ordering
    /// on the borrowed form *must* match the ordering on the key type.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use rune::alloc::BTreeMap;
    ///
    /// let mut map = BTreeMap::new();
    /// map.try_insert(1, "a")?;
    /// assert_eq!(map.remove_entry(&1), Some((1, "a")));
    /// assert_eq!(map.remove_entry(&1), None);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn remove_entry<Q: ?Sized>(&mut self, key: &Q) -> Option<(K, V)>
    where
        Q: Ord,
        K: Borrow<Q> + Ord,
    {
        into_ok(self.remove_entry_with(&mut (), key, infallible_cmp))
    }

    pub(crate) fn remove_entry_with<C: ?Sized, Q: ?Sized, E>(
        &mut self,
        cx: &mut C,
        key: &Q,
        cmp: CmpFn<C, Q, E>,
    ) -> Result<Option<(K, V)>, E>
    where
        K: Borrow<Q>,
    {
        let (map, dormant_map) = DormantMutRef::new(self);

        let Some(root_node) = map.root.as_mut().map(NodeRef::borrow_mut) else {
            return Ok(None);
        };

        Ok(match root_node.search_tree(cx, key, cmp)? {
            Found(handle) => {
                let entry = OccupiedEntry {
                    handle,
                    dormant_map,
                    alloc: &*map.alloc,
                    _marker: PhantomData,
                };

                Some(entry.remove_entry())
            }
            GoDown(_) => None,
        })
    }

    /// Retains only the elements specified by the predicate.
    ///
    /// In other words, remove all pairs `(k, v)` for which `f(&k, &mut v)`
    /// returns `false`. The elements are visited in ascending key order.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::BTreeMap;
    /// use rune::alloc::prelude::*;
    ///
    /// let mut map: BTreeMap<i32, i32> = (0..8).map(|x| (x, x*10)).try_collect()?;
    /// // Keep only the elements with even-numbered keys.
    /// map.retain(|&k, _| k % 2 == 0);
    /// assert!(map.into_iter().eq(vec![(0, 0), (2, 20), (4, 40), (6, 60)]));
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    pub fn retain<F>(&mut self, mut f: F)
    where
        K: Ord,
        F: FnMut(&K, &mut V) -> bool,
    {
        self.extract_if(|k, v| !f(k, v)).for_each(drop);
    }

    /// Moves all elements from `other` into `self`, leaving `other` empty.
    ///
    /// If a key from `other` is already present in `self`, the respective
    /// value from `self` will be overwritten with the respective value from `other`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::BTreeMap;
    ///
    /// let mut a = BTreeMap::new();
    /// a.try_insert(1, "a")?;
    /// a.try_insert(2, "b")?;
    /// a.try_insert(3, "c")?; // Note: Key (3) also present in b.
    ///
    /// let mut b = BTreeMap::new();
    /// b.try_insert(3, "d")?; // Note: Key (3) also present in a.
    /// b.try_insert(4, "e")?;
    /// b.try_insert(5, "f")?;
    ///
    /// a.try_append(&mut b);
    ///
    /// assert_eq!(a.len(), 5);
    /// assert_eq!(b.len(), 0);
    ///
    /// assert_eq!(a[&1], "a");
    /// assert_eq!(a[&2], "b");
    /// assert_eq!(a[&3], "d"); // Note: "c" has been overwritten.
    /// assert_eq!(a[&4], "e");
    /// assert_eq!(a[&5], "f");
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn try_append(&mut self, other: &mut Self) -> Result<(), AllocError>
    where
        K: Ord,
    {
        // Do we have to append anything at all?
        if other.is_empty() {
            return Ok(());
        }

        // We can just swap `self` and `other` if `self` is empty.
        if self.is_empty() {
            mem::swap(self, other);
            return Ok(());
        }

        let self_iter = into_iter!(self);
        let other_iter = into_iter!(other);

        let root = match &mut self.root {
            Some(root) => root,
            None => self.root.insert(Root::new(&*self.alloc)?),
        };

        root.try_append_from_sorted_iters(self_iter, other_iter, &mut self.length, &*self.alloc)
    }

    #[cfg(test)]
    pub(crate) fn append(&mut self, other: &mut Self)
    where
        K: Ord,
    {
        self.try_append(other).abort()
    }

    /// Constructs a double-ended iterator over a sub-range of elements in the map.
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
    /// Basic usage:
    ///
    /// ```
    /// use rune::alloc::BTreeMap;
    /// use core::ops::Bound::Included;
    ///
    /// let mut map = BTreeMap::new();
    /// map.try_insert(3, "a")?;
    /// map.try_insert(5, "b")?;
    /// map.try_insert(8, "c")?;
    ///
    /// for (&key, &value) in map.range((Included(&4), Included(&8))) {
    ///     println!("{key}: {value}");
    /// }
    ///
    /// assert_eq!(Some((&5, &"b")), map.range(4..).next());
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn range<Q: ?Sized, R>(&self, range: R) -> Range<'_, K, V>
    where
        Q: Ord,
        K: Borrow<Q> + Ord,
        R: RangeBounds<Q>,
    {
        into_ok(self.range_with(&mut (), range, infallible_cmp))
    }

    pub(crate) fn range_with<C: ?Sized, Q: ?Sized, R, E>(
        &self,
        cx: &mut C,
        range: R,
        cmp: CmpFn<C, Q, E>,
    ) -> Result<Range<'_, K, V>, E>
    where
        K: Borrow<Q>,
        R: RangeBounds<Q>,
    {
        Ok(if let Some(root) = &self.root {
            Range {
                inner: root.reborrow().range_search(cx, range, cmp)?,
            }
        } else {
            Range {
                inner: LeafRange::none(),
            }
        })
    }

    /// Constructs a mutable double-ended iterator over a sub-range of elements in the map.
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
    /// Basic usage:
    ///
    /// ```
    /// use rune::alloc::BTreeMap;
    ///
    /// let mut map: BTreeMap<&str, i32> =
    ///     [("Alice", 0), ("Bob", 0), ("Carol", 0), ("Cheryl", 0)].try_into()?;
    ///
    /// for (_, balance) in map.range_mut("B".."Cheryl") {
    ///     *balance += 100;
    /// }
    ///
    /// for (name, balance) in &map {
    ///     println!("{name} => {balance}");
    /// }
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn range_mut<Q: ?Sized, R>(&mut self, range: R) -> RangeMut<'_, K, V>
    where
        Q: Ord,
        K: Borrow<Q> + Ord,
        R: RangeBounds<Q>,
    {
        into_ok(self.range_mut_with(&mut (), range, infallible_cmp))
    }

    pub(crate) fn range_mut_with<C: ?Sized, Q: ?Sized, R, E>(
        &mut self,
        cx: &mut C,
        range: R,
        cmp: CmpFn<C, Q, E>,
    ) -> Result<RangeMut<'_, K, V>, E>
    where
        K: Borrow<Q>,
        R: RangeBounds<Q>,
    {
        Ok(if let Some(root) = &mut self.root {
            RangeMut {
                inner: root.borrow_valmut().range_search(cx, range, cmp)?,
                _marker: PhantomData,
            }
        } else {
            RangeMut {
                inner: LeafRange::none(),
                _marker: PhantomData,
            }
        })
    }

    /// Gets the given key's corresponding entry in the map for in-place
    /// manipulation.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use rune::alloc::BTreeMap;
    ///
    /// let mut count: BTreeMap<&str, usize> = BTreeMap::new();
    ///
    /// // count the number of occurrences of letters in the vec
    /// for x in ["a", "b", "a", "c", "a", "b"] {
    ///     count.entry(x).and_modify(|curr| *curr += 1).or_try_insert(1)?;
    /// }
    ///
    /// assert_eq!(count["a"], 3);
    /// assert_eq!(count["b"], 2);
    /// assert_eq!(count["c"], 1);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn entry(&mut self, key: K) -> Entry<'_, K, V, A>
    where
        K: Ord,
    {
        into_ok(self.entry_with(&mut (), key, infallible_cmp))
    }

    pub(crate) fn entry_with<C: ?Sized, E>(
        &mut self,
        cx: &mut C,
        key: K,
        cmp: CmpFn<C, K, E>,
    ) -> Result<Entry<'_, K, V, A>, E> {
        let (map, dormant_map) = DormantMutRef::new(self);

        Ok(match map.root {
            None => Vacant(VacantEntry {
                key,
                handle: None,
                dormant_map,
                alloc: &*map.alloc,
                _marker: PhantomData,
            }),

            Some(ref mut root) => match root.borrow_mut().search_tree(cx, &key, cmp)? {
                Found(handle) => Occupied(OccupiedEntry {
                    handle,
                    dormant_map,
                    alloc: &*map.alloc,
                    _marker: PhantomData,
                }),
                GoDown(handle) => Vacant(VacantEntry {
                    key,
                    handle: Some(handle),
                    dormant_map,
                    alloc: &*map.alloc,
                    _marker: PhantomData,
                }),
            },
        })
    }

    /// Splits the collection into two at the given key. Returns everything after the given key,
    /// including the key.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use rune::alloc::BTreeMap;
    ///
    /// let mut a = BTreeMap::new();
    /// a.try_insert(1, "a")?;
    /// a.try_insert(2, "b")?;
    /// a.try_insert(3, "c")?;
    /// a.try_insert(17, "d")?;
    /// a.try_insert(41, "e")?;
    ///
    /// let b = a.try_split_off(&3)?;
    ///
    /// assert_eq!(a.len(), 2);
    /// assert_eq!(b.len(), 3);
    ///
    /// assert_eq!(a[&1], "a");
    /// assert_eq!(a[&2], "b");
    ///
    /// assert_eq!(b[&3], "c");
    /// assert_eq!(b[&17], "d");
    /// assert_eq!(b[&41], "e");
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn try_split_off<Q: ?Sized>(&mut self, key: &Q) -> Result<Self, Error>
    where
        Q: Ord,
        K: Borrow<Q> + Ord,
        A: Clone,
    {
        into_ok(self.try_split_off_with(&mut (), key, infallible_cmp))
    }

    #[cfg(test)]
    pub(crate) fn split_off<Q: ?Sized>(&mut self, key: &Q) -> Self
    where
        Q: Ord,
        K: Borrow<Q> + Ord,
        A: Clone,
    {
        self.try_split_off(key).abort()
    }

    pub(crate) fn try_split_off_with<C: ?Sized, Q: ?Sized, E>(
        &mut self,
        cx: &mut C,
        key: &Q,
        cmp: CmpFn<C, Q, E>,
    ) -> Result<Result<Self, Error>, E>
    where
        K: Borrow<Q>,
        A: Clone,
    {
        if self.is_empty() {
            return Ok(Ok(Self::new_in((*self.alloc).clone())));
        }

        let total_num = self.len();
        let left_root = self.root.as_mut().unwrap(); // unwrap succeeds because not empty

        let right_root = match left_root.split_off(cx, key, &*self.alloc, cmp)? {
            Ok(right_root) => right_root,
            Err(error) => return Ok(Err(Error::from(error))),
        };

        let (new_left_len, right_len) = Root::calc_split_length(total_num, left_root, &right_root);
        self.length = new_left_len;

        Ok(Ok(BTreeMap {
            root: Some(right_root),
            length: right_len,
            alloc: self.alloc.clone(),
            _marker: PhantomData,
        }))
    }

    /// Creates an iterator that visits all elements (key-value pairs) in
    /// ascending key order and uses a closure to determine if an element should
    /// be removed. If the closure returns `true`, the element is removed from
    /// the map and yielded. If the closure returns `false`, or panics, the
    /// element remains in the map and will not be yielded.
    ///
    /// The iterator also lets you mutate the value of each element in the
    /// closure, regardless of whether you choose to keep or remove it.
    ///
    /// If the returned `ExtractIf` is not exhausted, e.g. because it is dropped without iterating
    /// or the iteration short-circuits, then the remaining elements will be retained.
    /// Use [`retain`] with a negated predicate if you do not need the returned iterator.
    ///
    /// [`retain`]: BTreeMap::retain
    ///
    /// # Examples
    ///
    /// Splitting a map into even and odd keys, reusing the original map:
    ///
    /// ```
    /// use rune::alloc::{Vec, BTreeMap};
    /// use rune::alloc::prelude::*;
    ///
    /// let mut map: BTreeMap<i32, i32> = (0..8).map(|x| (x, x)).try_collect()?;
    /// let evens: BTreeMap<_, _> = map.extract_if(|k, _v| k % 2 == 0).try_collect()?;
    /// let odds = map;
    /// assert_eq!(evens.keys().copied().try_collect::<Vec<_>>()?, [0, 2, 4, 6]);
    /// assert_eq!(odds.keys().copied().try_collect::<Vec<_>>()?, [1, 3, 5, 7]);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn extract_if<F>(&mut self, pred: F) -> ExtractIf<'_, K, V, F, A>
    where
        F: FnMut(&K, &mut V) -> bool,
    {
        let (inner, alloc) = self.extract_if_inner();
        ExtractIf { pred, inner, alloc }
    }

    pub(super) fn extract_if_inner(&mut self) -> (ExtractIfInner<'_, K, V>, &A) {
        if let Some(root) = self.root.as_mut() {
            let (root, dormant_root) = DormantMutRef::new(root);
            let front = root.borrow_mut().first_leaf_edge();
            (
                ExtractIfInner {
                    length: &mut self.length,
                    dormant_root: Some(dormant_root),
                    cur_leaf_edge: Some(front),
                },
                &self.alloc,
            )
        } else {
            (
                ExtractIfInner {
                    length: &mut self.length,
                    dormant_root: None,
                    cur_leaf_edge: None,
                },
                &self.alloc,
            )
        }
    }

    /// Creates a consuming iterator visiting all the keys, in sorted order. The
    /// map cannot be used after calling this. The iterator element type is `K`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::{BTreeMap, Vec};
    /// use rune::alloc::prelude::*;
    ///
    /// let mut a = BTreeMap::new();
    /// a.try_insert(2, "b")?;
    /// a.try_insert(1, "a")?;
    ///
    /// let keys: Vec<i32> = a.into_keys().try_collect()?;
    /// assert_eq!(keys, [1, 2]);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    pub fn into_keys(self) -> IntoKeys<K, V, A> {
        IntoKeys {
            inner: self.into_iter(),
        }
    }

    /// Creates a consuming iterator visiting all the values, in order by key.
    /// The map cannot be used after calling this. The iterator element type is
    /// `V`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::{BTreeMap, Vec};
    /// use rune::alloc::prelude::*;
    ///
    /// let mut a = BTreeMap::new();
    /// a.try_insert(1, "hello");
    /// a.try_insert(2, "goodbye");
    ///
    /// let values: Vec<&str> = a.into_values().try_collect()?;
    /// assert_eq!(values, ["hello", "goodbye"]);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    pub fn into_values(self) -> IntoValues<K, V, A> {
        IntoValues {
            inner: self.into_iter(),
        }
    }
}

impl<'a, K, V, A: Allocator> IntoIterator for &'a BTreeMap<K, V, A> {
    type Item = (&'a K, &'a V);
    type IntoIter = Iter<'a, K, V>;

    fn into_iter(self) -> Iter<'a, K, V> {
        self.iter()
    }
}

impl<'a, K: 'a, V: 'a> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<(&'a K, &'a V)> {
        if self.length == 0 {
            None
        } else {
            self.length -= 1;
            Some(unsafe { self.range.next_unchecked() })
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.length, Some(self.length))
    }

    fn last(mut self) -> Option<(&'a K, &'a V)> {
        self.next_back()
    }

    fn min(mut self) -> Option<(&'a K, &'a V)>
    where
        (&'a K, &'a V): Ord,
    {
        self.next()
    }

    fn max(mut self) -> Option<(&'a K, &'a V)>
    where
        (&'a K, &'a V): Ord,
    {
        self.next_back()
    }
}

impl<K, V> FusedIterator for Iter<'_, K, V> {}

impl<'a, K: 'a, V: 'a> DoubleEndedIterator for Iter<'a, K, V> {
    fn next_back(&mut self) -> Option<(&'a K, &'a V)> {
        if self.length == 0 {
            None
        } else {
            self.length -= 1;
            Some(unsafe { self.range.next_back_unchecked() })
        }
    }
}

impl<K, V> ExactSizeIterator for Iter<'_, K, V> {
    fn len(&self) -> usize {
        self.length
    }
}

impl<K, V> Clone for Iter<'_, K, V> {
    fn clone(&self) -> Self {
        Iter {
            range: self.range.clone(),
            length: self.length,
        }
    }
}

impl<'a, K, V, A: Allocator> IntoIterator for &'a mut BTreeMap<K, V, A> {
    type Item = (&'a K, &'a mut V);
    type IntoIter = IterMut<'a, K, V>;

    fn into_iter(self) -> IterMut<'a, K, V> {
        self.iter_mut()
    }
}

impl<'a, K, V> Iterator for IterMut<'a, K, V> {
    type Item = (&'a K, &'a mut V);

    fn next(&mut self) -> Option<(&'a K, &'a mut V)> {
        if self.length == 0 {
            None
        } else {
            self.length -= 1;
            Some(unsafe { self.range.next_unchecked() })
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.length, Some(self.length))
    }

    fn last(mut self) -> Option<(&'a K, &'a mut V)> {
        self.next_back()
    }

    fn min(mut self) -> Option<(&'a K, &'a mut V)>
    where
        (&'a K, &'a mut V): Ord,
    {
        self.next()
    }

    fn max(mut self) -> Option<(&'a K, &'a mut V)>
    where
        (&'a K, &'a mut V): Ord,
    {
        self.next_back()
    }
}

impl<'a, K, V> DoubleEndedIterator for IterMut<'a, K, V> {
    fn next_back(&mut self) -> Option<(&'a K, &'a mut V)> {
        if self.length == 0 {
            None
        } else {
            self.length -= 1;
            Some(unsafe { self.range.next_back_unchecked() })
        }
    }
}

impl<K, V> ExactSizeIterator for IterMut<'_, K, V> {
    fn len(&self) -> usize {
        self.length
    }
}

impl<K, V> FusedIterator for IterMut<'_, K, V> {}

impl<'a, K, V> IterMut<'a, K, V> {
    /// Returns an iterator of references over the remaining items.
    #[inline]
    pub(super) fn iter(&self) -> Iter<'_, K, V> {
        Iter {
            range: self.range.reborrow(),
            length: self.length,
        }
    }
}

impl<K, V, A: Allocator> IntoIterator for BTreeMap<K, V, A> {
    type Item = (K, V);
    type IntoIter = IntoIter<K, V, A>;

    fn into_iter(self) -> IntoIter<K, V, A> {
        let mut me = ManuallyDrop::new(self);
        if let Some(root) = me.root.take() {
            let full_range = root.into_dying().full_range();

            IntoIter {
                range: full_range,
                length: me.length,
                alloc: unsafe { ManuallyDrop::take(&mut me.alloc) },
            }
        } else {
            IntoIter {
                range: LazyLeafRange::none(),
                length: 0,
                alloc: unsafe { ManuallyDrop::take(&mut me.alloc) },
            }
        }
    }
}

impl<K, V, A: Allocator> Drop for IntoIter<K, V, A> {
    fn drop(&mut self) {
        struct DropGuard<'a, K, V, A: Allocator>(&'a mut IntoIter<K, V, A>);

        impl<'a, K, V, A: Allocator> Drop for DropGuard<'a, K, V, A> {
            fn drop(&mut self) {
                // Continue the same loop we perform below. This only runs when unwinding, so we
                // don't have to care about panics this time (they'll abort).
                while let Some(kv) = self.0.dying_next() {
                    // SAFETY: we consume the dying handle immediately.
                    unsafe { kv.drop_key_val() };
                }
            }
        }

        while let Some(kv) = self.dying_next() {
            let guard = DropGuard(self);
            // SAFETY: we don't touch the tree before consuming the dying handle.
            unsafe { kv.drop_key_val() };
            mem::forget(guard);
        }
    }
}

impl<K, V, A: Allocator> IntoIter<K, V, A> {
    /// Core of a `next` method returning a dying KV handle,
    /// invalidated by further calls to this function and some others.
    fn dying_next(
        &mut self,
    ) -> Option<Handle<NodeRef<marker::Dying, K, V, marker::LeafOrInternal>, marker::KV>> {
        if self.length == 0 {
            self.range.deallocating_end(&self.alloc);
            None
        } else {
            self.length -= 1;
            Some(unsafe { self.range.deallocating_next_unchecked(&self.alloc) })
        }
    }

    /// Core of a `next_back` method returning a dying KV handle,
    /// invalidated by further calls to this function and some others.
    fn dying_next_back(
        &mut self,
    ) -> Option<Handle<NodeRef<marker::Dying, K, V, marker::LeafOrInternal>, marker::KV>> {
        if self.length == 0 {
            self.range.deallocating_end(&self.alloc);
            None
        } else {
            self.length -= 1;
            Some(unsafe { self.range.deallocating_next_back_unchecked(&self.alloc) })
        }
    }
}

impl<K, V, A: Allocator> Iterator for IntoIter<K, V, A> {
    type Item = (K, V);

    fn next(&mut self) -> Option<(K, V)> {
        // SAFETY: we consume the dying handle immediately.
        self.dying_next().map(unsafe { |kv| kv.into_key_val() })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.length, Some(self.length))
    }
}

impl<K, V, A: Allocator> DoubleEndedIterator for IntoIter<K, V, A> {
    fn next_back(&mut self) -> Option<(K, V)> {
        // SAFETY: we consume the dying handle immediately.
        self.dying_next_back()
            .map(unsafe { |kv| kv.into_key_val() })
    }
}

impl<K, V, A: Allocator> ExactSizeIterator for IntoIter<K, V, A> {
    fn len(&self) -> usize {
        self.length
    }
}

impl<K, V, A: Allocator> FusedIterator for IntoIter<K, V, A> {}

impl<'a, K, V> Iterator for Keys<'a, K, V> {
    type Item = &'a K;

    fn next(&mut self) -> Option<&'a K> {
        self.inner.next().map(|(k, _)| k)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }

    fn last(mut self) -> Option<&'a K> {
        self.next_back()
    }

    fn min(mut self) -> Option<&'a K>
    where
        &'a K: Ord,
    {
        self.next()
    }

    fn max(mut self) -> Option<&'a K>
    where
        &'a K: Ord,
    {
        self.next_back()
    }
}

impl<'a, K, V> DoubleEndedIterator for Keys<'a, K, V> {
    fn next_back(&mut self) -> Option<&'a K> {
        self.inner.next_back().map(|(k, _)| k)
    }
}

impl<K, V> ExactSizeIterator for Keys<'_, K, V> {
    fn len(&self) -> usize {
        self.inner.len()
    }
}

impl<K, V> FusedIterator for Keys<'_, K, V> {}

impl<K, V> Clone for Keys<'_, K, V> {
    fn clone(&self) -> Self {
        Keys {
            inner: self.inner.clone(),
        }
    }
}

impl<K, V> Default for Keys<'_, K, V> {
    /// Creates an empty `btree_map::Keys`.
    ///
    /// ```
    /// use rune::alloc::btree_map;
    ///
    /// let iter: btree_map::Keys<'_, u8, u8> = Default::default();
    /// assert_eq!(iter.len(), 0);
    /// ```
    fn default() -> Self {
        Keys {
            inner: Default::default(),
        }
    }
}

impl<'a, K, V> Iterator for Values<'a, K, V> {
    type Item = &'a V;

    fn next(&mut self) -> Option<&'a V> {
        self.inner.next().map(|(_, v)| v)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }

    fn last(mut self) -> Option<&'a V> {
        self.next_back()
    }
}

impl<'a, K, V> DoubleEndedIterator for Values<'a, K, V> {
    fn next_back(&mut self) -> Option<&'a V> {
        self.inner.next_back().map(|(_, v)| v)
    }
}

impl<K, V> ExactSizeIterator for Values<'_, K, V> {
    fn len(&self) -> usize {
        self.inner.len()
    }
}

impl<K, V> FusedIterator for Values<'_, K, V> {}

impl<K, V> Clone for Values<'_, K, V> {
    fn clone(&self) -> Self {
        Values {
            inner: self.inner.clone(),
        }
    }
}

impl<K, V> Default for Values<'_, K, V> {
    /// Creates an empty `btree_map::Values`.
    ///
    /// ```
    /// use rune::alloc::btree_map;
    ///
    /// let iter: btree_map::Values<'_, u8, u8> = Default::default();
    /// assert_eq!(iter.len(), 0);
    /// ```
    fn default() -> Self {
        Values {
            inner: Default::default(),
        }
    }
}

/// An iterator produced by calling `extract_if` on BTreeMap.
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct ExtractIf<'a, K, V, F, A: Allocator = Global>
where
    F: 'a + FnMut(&K, &mut V) -> bool,
{
    pred: F,
    inner: ExtractIfInner<'a, K, V>,
    /// The BTreeMap will outlive this IntoIter so we don't care about drop order for `alloc`.
    alloc: &'a A,
}

/// Most of the implementation of ExtractIf are generic over the type
/// of the predicate, thus also serving for BTreeSet::ExtractIf.
pub(super) struct ExtractIfInner<'a, K, V> {
    /// Reference to the length field in the borrowed map, updated live.
    length: &'a mut usize,
    /// Buried reference to the root field in the borrowed map.
    /// Wrapped in `Option` to allow drop handler to `take` it.
    dormant_root: Option<DormantMutRef<'a, Root<K, V>>>,
    /// Contains a leaf edge preceding the next element to be returned, or the last leaf edge.
    /// Empty if the map has no root, if iteration went beyond the last leaf edge,
    /// or if a panic occurred in the predicate.
    cur_leaf_edge: Option<Handle<NodeRef<marker::Mut<'a>, K, V, marker::Leaf>, marker::Edge>>,
}

impl<K, V, F> fmt::Debug for ExtractIf<'_, K, V, F>
where
    K: fmt::Debug,
    V: fmt::Debug,
    F: FnMut(&K, &mut V) -> bool,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ExtractIf")
            .field(&self.inner.peek())
            .finish()
    }
}

impl<K, V, F, A: Allocator> Iterator for ExtractIf<'_, K, V, F, A>
where
    F: FnMut(&K, &mut V) -> bool,
{
    type Item = (K, V);

    fn next(&mut self) -> Option<(K, V)> {
        self.inner.next(&mut self.pred, self.alloc)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<'a, K, V> ExtractIfInner<'a, K, V> {
    /// Allow Debug implementations to predict the next element.
    pub(super) fn peek(&self) -> Option<(&K, &V)> {
        let edge = self.cur_leaf_edge.as_ref()?;
        edge.reborrow().next_kv().ok().map(Handle::into_kv)
    }

    /// Implementation of a typical `ExtractIf::next` method, given the predicate.
    pub(super) fn next<F, A: Allocator>(&mut self, pred: &mut F, alloc: &A) -> Option<(K, V)>
    where
        F: FnMut(&K, &mut V) -> bool,
    {
        while let Ok(mut kv) = self.cur_leaf_edge.take()?.next_kv() {
            let (k, v) = kv.kv_mut();
            if pred(k, v) {
                *self.length -= 1;
                let (kv, pos) = kv.remove_kv_tracking(
                    || {
                        // SAFETY: we will touch the root in a way that will not
                        // invalidate the position returned.
                        let root = unsafe { self.dormant_root.take().unwrap().awaken() };
                        root.pop_internal_level(alloc);
                        self.dormant_root = Some(DormantMutRef::new(root).1);
                    },
                    alloc,
                );
                self.cur_leaf_edge = Some(pos);
                return Some(kv);
            }
            self.cur_leaf_edge = Some(kv.next_leaf_edge());
        }
        None
    }

    /// Implementation of a typical `ExtractIf::size_hint` method.
    pub(super) fn size_hint(&self) -> (usize, Option<usize>) {
        // In most of the btree iterators, `self.length` is the number of elements
        // yet to be visited. Here, it includes elements that were visited and that
        // the predicate decided not to drain. Making this upper bound more tight
        // during iteration would require an extra field.
        (0, Some(*self.length))
    }
}

impl<K, V, F> FusedIterator for ExtractIf<'_, K, V, F> where F: FnMut(&K, &mut V) -> bool {}

impl<'a, K, V> Iterator for Range<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<(&'a K, &'a V)> {
        self.inner.next_checked()
    }

    fn last(mut self) -> Option<(&'a K, &'a V)> {
        self.next_back()
    }

    fn min(mut self) -> Option<(&'a K, &'a V)>
    where
        (&'a K, &'a V): Ord,
    {
        self.next()
    }

    fn max(mut self) -> Option<(&'a K, &'a V)>
    where
        (&'a K, &'a V): Ord,
    {
        self.next_back()
    }
}

impl<K, V> Default for Range<'_, K, V> {
    /// Creates an empty [`Range`].
    ///
    /// ```
    /// use rune::alloc::btree_map;
    ///
    /// let iter: btree_map::Range<'_, u8, u8> = Default::default();
    /// assert_eq!(iter.count(), 0);
    /// ```
    fn default() -> Self {
        Range {
            inner: Default::default(),
        }
    }
}

impl<'a, K, V> Iterator for ValuesMut<'a, K, V> {
    type Item = &'a mut V;

    fn next(&mut self) -> Option<&'a mut V> {
        self.inner.next().map(|(_, v)| v)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }

    fn last(mut self) -> Option<&'a mut V> {
        self.next_back()
    }
}

impl<'a, K, V> DoubleEndedIterator for ValuesMut<'a, K, V> {
    fn next_back(&mut self) -> Option<&'a mut V> {
        self.inner.next_back().map(|(_, v)| v)
    }
}

impl<K, V> ExactSizeIterator for ValuesMut<'_, K, V> {
    fn len(&self) -> usize {
        self.inner.len()
    }
}

impl<K, V> FusedIterator for ValuesMut<'_, K, V> {}

impl<K, V, A: Allocator> Iterator for IntoKeys<K, V, A> {
    type Item = K;

    fn next(&mut self) -> Option<K> {
        self.inner.next().map(|(k, _)| k)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }

    fn last(mut self) -> Option<K> {
        self.next_back()
    }

    fn min(mut self) -> Option<K>
    where
        K: Ord,
    {
        self.next()
    }

    fn max(mut self) -> Option<K>
    where
        K: Ord,
    {
        self.next_back()
    }
}

impl<K, V, A: Allocator> DoubleEndedIterator for IntoKeys<K, V, A> {
    fn next_back(&mut self) -> Option<K> {
        self.inner.next_back().map(|(k, _)| k)
    }
}

impl<K, V, A: Allocator> ExactSizeIterator for IntoKeys<K, V, A> {
    fn len(&self) -> usize {
        self.inner.len()
    }
}

impl<K, V, A: Allocator> FusedIterator for IntoKeys<K, V, A> {}

impl<K, V, A> Default for IntoKeys<K, V, A>
where
    A: Allocator + Default + Clone,
{
    /// Creates an empty `btree_map::IntoKeys`.
    ///
    /// ```
    /// use rune::alloc::btree_map;
    ///
    /// let iter: btree_map::IntoKeys<u8, u8> = Default::default();
    /// assert_eq!(iter.len(), 0);
    /// ```
    fn default() -> Self {
        IntoKeys {
            inner: Default::default(),
        }
    }
}

impl<K, V, A: Allocator> Iterator for IntoValues<K, V, A> {
    type Item = V;

    fn next(&mut self) -> Option<V> {
        self.inner.next().map(|(_, v)| v)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }

    fn last(mut self) -> Option<V> {
        self.next_back()
    }
}

impl<K, V, A: Allocator> DoubleEndedIterator for IntoValues<K, V, A> {
    fn next_back(&mut self) -> Option<V> {
        self.inner.next_back().map(|(_, v)| v)
    }
}

impl<K, V, A: Allocator> ExactSizeIterator for IntoValues<K, V, A> {
    fn len(&self) -> usize {
        self.inner.len()
    }
}

impl<K, V, A: Allocator> FusedIterator for IntoValues<K, V, A> {}

impl<K, V, A> Default for IntoValues<K, V, A>
where
    A: Allocator + Default + Clone,
{
    /// Creates an empty `btree_map::IntoValues`.
    ///
    /// ```
    /// use rune::alloc::btree_map;
    ///
    /// let iter: btree_map::IntoValues<u8, u8> = Default::default();
    /// assert_eq!(iter.len(), 0);
    /// ```
    fn default() -> Self {
        IntoValues {
            inner: Default::default(),
        }
    }
}

impl<'a, K, V> DoubleEndedIterator for Range<'a, K, V> {
    fn next_back(&mut self) -> Option<(&'a K, &'a V)> {
        self.inner.next_back_checked()
    }
}

impl<K, V> FusedIterator for Range<'_, K, V> {}

impl<K, V> Clone for Range<'_, K, V> {
    fn clone(&self) -> Self {
        Range {
            inner: self.inner.clone(),
        }
    }
}

impl<'a, K, V> Iterator for RangeMut<'a, K, V> {
    type Item = (&'a K, &'a mut V);

    fn next(&mut self) -> Option<(&'a K, &'a mut V)> {
        self.inner.next_checked()
    }

    fn last(mut self) -> Option<(&'a K, &'a mut V)> {
        self.next_back()
    }

    fn min(mut self) -> Option<(&'a K, &'a mut V)>
    where
        (&'a K, &'a mut V): Ord,
    {
        self.next()
    }

    fn max(mut self) -> Option<(&'a K, &'a mut V)>
    where
        (&'a K, &'a mut V): Ord,
    {
        self.next_back()
    }
}

impl<'a, K, V> DoubleEndedIterator for RangeMut<'a, K, V> {
    fn next_back(&mut self) -> Option<(&'a K, &'a mut V)> {
        self.inner.next_back_checked()
    }
}

impl<K, V> FusedIterator for RangeMut<'_, K, V> {}

impl<K: Ord, V, A: Allocator + Clone> TryExtend<(K, V)> for BTreeMap<K, V, A> {
    #[inline]
    fn try_extend<T: IntoIterator<Item = (K, V)>>(&mut self, iter: T) -> Result<(), Error> {
        for (k, v) in iter {
            self.try_insert(k, v)?;
        }

        Ok(())
    }
}

#[cfg(test)]
impl<K: Ord, V, A: Allocator + Clone> Extend<(K, V)> for BTreeMap<K, V, A> {
    #[inline]
    fn extend<T: IntoIterator<Item = (K, V)>>(&mut self, iter: T) {
        self.try_extend(iter).abort();
    }
}

impl<'a, K: Ord + Copy, V: Copy, A: Allocator + Clone> TryExtend<(&'a K, &'a V)>
    for BTreeMap<K, V, A>
{
    fn try_extend<I: IntoIterator<Item = (&'a K, &'a V)>>(&mut self, iter: I) -> Result<(), Error> {
        self.try_extend(iter.into_iter().map(|(&key, &value)| (key, value)))
    }
}

#[cfg(test)]
impl<'a, K: Ord + Copy, V: Copy, A: Allocator + Clone> Extend<(&'a K, &'a V)>
    for BTreeMap<K, V, A>
{
    fn extend<I: IntoIterator<Item = (&'a K, &'a V)>>(&mut self, iter: I) {
        self.try_extend(iter).abort();
    }
}

impl<K: Hash, V: Hash, A: Allocator> Hash for BTreeMap<K, V, A> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_usize(self.len());
        for elt in self {
            elt.hash(state);
        }
    }
}

impl<K, V> Default for BTreeMap<K, V> {
    /// Creates an empty `BTreeMap`.
    fn default() -> BTreeMap<K, V> {
        BTreeMap::new()
    }
}

impl<K: PartialEq, V: PartialEq, A: Allocator> PartialEq for BTreeMap<K, V, A> {
    fn eq(&self, other: &BTreeMap<K, V, A>) -> bool {
        self.len() == other.len() && self.iter().zip(other).all(|(a, b)| a == b)
    }
}

impl<K: Eq, V: Eq, A: Allocator> Eq for BTreeMap<K, V, A> {}

impl<K: PartialOrd, V: PartialOrd, A: Allocator> PartialOrd for BTreeMap<K, V, A> {
    #[inline]
    fn partial_cmp(&self, other: &BTreeMap<K, V, A>) -> Option<Ordering> {
        self.iter().partial_cmp(other.iter())
    }
}

impl<K: Ord, V: Ord, A: Allocator> Ord for BTreeMap<K, V, A> {
    #[inline]
    fn cmp(&self, other: &BTreeMap<K, V, A>) -> Ordering {
        self.iter().cmp(other.iter())
    }
}

impl<K: Debug, V: Debug, A: Allocator> Debug for BTreeMap<K, V, A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

impl<K, Q: ?Sized, V, A: Allocator> Index<&Q> for BTreeMap<K, V, A>
where
    K: Borrow<Q> + Ord,
    Q: Ord,
{
    type Output = V;

    /// Returns a reference to the value corresponding to the supplied key.
    ///
    /// # Panics
    ///
    /// Panics if the key is not present in the `BTreeMap`.
    #[inline]
    fn index(&self, key: &Q) -> &V {
        self.get(key).expect("no entry found for key")
    }
}

impl<K, V, A: Allocator> BTreeMap<K, V, A> {
    /// Gets an iterator over the entries of the map, sorted by key.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use rune::alloc::BTreeMap;
    ///
    /// let mut map = BTreeMap::new();
    /// map.try_insert(3, "c")?;
    /// map.try_insert(2, "b")?;
    /// map.try_insert(1, "a")?;
    ///
    /// for (key, value) in map.iter() {
    ///     println!("{key}: {value}");
    /// }
    ///
    /// let (first_key, first_value) = map.iter().next().unwrap();
    /// assert_eq!((*first_key, *first_value), (1, "a"));
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn iter(&self) -> Iter<'_, K, V> {
        if let Some(root) = &self.root {
            let full_range = root.reborrow().full_range();

            Iter {
                range: full_range,
                length: self.length,
            }
        } else {
            Iter {
                range: LazyLeafRange::none(),
                length: 0,
            }
        }
    }

    /// Perform a raw iteration over the btree.
    ///
    /// # Safety
    ///
    /// Caller must ensure that the returned iterator doesn't outlive `self`.
    pub unsafe fn iter_raw(&self) -> IterRaw<K, V> {
        if let Some(root) = &self.root {
            let full_range = root.raw().full_range();

            IterRaw {
                range: full_range,
                length: self.length,
            }
        } else {
            IterRaw {
                range: LazyLeafRange::none(),
                length: 0,
            }
        }
    }

    /// Gets a mutable iterator over the entries of the map, sorted by key.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use rune::alloc::BTreeMap;
    ///
    /// let mut map = BTreeMap::try_from([
    ///    ("a", 1),
    ///    ("b", 2),
    ///    ("c", 3),
    /// ])?;
    ///
    /// // add 10 to the value if the key isn't "a"
    /// for (key, value) in map.iter_mut() {
    ///     if key != &"a" {
    ///         *value += 10;
    ///     }
    /// }
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn iter_mut(&mut self) -> IterMut<'_, K, V> {
        if let Some(root) = &mut self.root {
            let full_range = root.borrow_valmut().full_range();

            IterMut {
                range: full_range,
                length: self.length,
                _marker: PhantomData,
            }
        } else {
            IterMut {
                range: LazyLeafRange::none(),
                length: 0,
                _marker: PhantomData,
            }
        }
    }

    /// Gets an iterator over the keys of the map, in sorted order.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use rune::alloc::BTreeMap;
    ///
    /// let mut a = BTreeMap::new();
    /// a.try_insert(2, "b")?;
    /// a.try_insert(1, "a")?;
    ///
    /// let keys: Vec<_> = a.keys().cloned().collect();
    /// assert_eq!(keys, [1, 2]);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn keys(&self) -> Keys<'_, K, V> {
        Keys { inner: self.iter() }
    }

    /// Gets an iterator over the values of the map, in order by key.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use rune::alloc::{BTreeMap, Vec};
    /// use rune::alloc::prelude::*;
    ///
    /// let mut a = BTreeMap::new();
    /// a.try_insert(1, "hello")?;
    /// a.try_insert(2, "goodbye")?;
    ///
    /// let values: Vec<&str> = a.values().copied().try_collect()?;
    /// assert_eq!(values, ["hello", "goodbye"]);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn values(&self) -> Values<'_, K, V> {
        Values { inner: self.iter() }
    }

    /// Gets a mutable iterator over the values of the map, in order by key.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use rune::alloc::{BTreeMap, Vec, String};
    /// use rune::alloc::prelude::*;
    ///
    /// let mut a = BTreeMap::new();
    /// a.try_insert(1, String::try_from("hello")?)?;
    /// a.try_insert(2, String::try_from("goodbye")?)?;
    ///
    /// for value in a.values_mut() {
    ///     value.try_push_str("!")?;
    /// }
    ///
    /// let mut values = Vec::new();
    ///
    /// for value in a.values() {
    ///     values.try_push(value.try_clone()?)?;
    /// }
    ///
    /// assert_eq!(values, [String::try_from("hello!")?,
    ///                     String::try_from("goodbye!")?]);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn values_mut(&mut self) -> ValuesMut<'_, K, V> {
        ValuesMut {
            inner: self.iter_mut(),
        }
    }

    /// Returns the number of elements in the map.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use rune::alloc::BTreeMap;
    ///
    /// let mut a = BTreeMap::new();
    /// assert_eq!(a.len(), 0);
    /// a.try_insert(1, "a")?;
    /// assert_eq!(a.len(), 1);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[must_use]
    pub const fn len(&self) -> usize {
        self.length
    }

    /// Returns `true` if the map contains no elements.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use rune::alloc::BTreeMap;
    ///
    /// let mut a = BTreeMap::new();
    /// assert!(a.is_empty());
    /// a.try_insert(1, "a")?;
    /// assert!(!a.is_empty());
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns a [`Cursor`] pointing at the first element that is above the
    /// given bound.
    ///
    /// If no such element exists then a cursor pointing at the "ghost"
    /// non-element is returned.
    ///
    /// Passing [`Bound::Unbounded`] will return a cursor pointing at the first
    /// element of the map.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use rune::alloc::BTreeMap;
    /// use std::ops::Bound;
    ///
    /// let mut a = BTreeMap::new();
    /// a.try_insert(1, "a")?;
    /// a.try_insert(2, "b")?;
    /// a.try_insert(3, "c")?;
    /// a.try_insert(4, "c")?;
    /// let cursor = a.lower_bound(Bound::Excluded(&2));
    /// assert_eq!(cursor.key(), Some(&3));
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn lower_bound<Q>(&self, bound: Bound<&Q>) -> Cursor<'_, K, V>
    where
        K: Borrow<Q> + Ord,
        Q: Ord,
    {
        into_ok(self.lower_bound_with(&mut (), bound, infallible_cmp))
    }

    pub(crate) fn lower_bound_with<C, Q, E>(
        &self,
        cx: &mut C,
        bound: Bound<&Q>,
        cmp: CmpFn<C, Q, E>,
    ) -> Result<Cursor<'_, K, V>, E>
    where
        K: Borrow<Q>,
    {
        let Some(root_node) = self.root.as_ref().map(NodeRef::reborrow) else {
            return Ok(Cursor {
                current: None,
                root: None,
            });
        };

        let edge = root_node.lower_bound(cx, SearchBound::from_range(bound), cmp)?;

        Ok(Cursor {
            current: edge.next_kv().ok(),
            root: self.root.as_ref(),
        })
    }

    /// Returns a [`CursorMut`] pointing at the first element that is above the
    /// given bound.
    ///
    /// If no such element exists then a cursor pointing at the "ghost"
    /// non-element is returned.
    ///
    /// Passing [`Bound::Unbounded`] will return a cursor pointing at the first
    /// element of the map.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use rune::alloc::BTreeMap;
    /// use std::ops::Bound;
    ///
    /// let mut a = BTreeMap::new();
    /// a.try_insert(1, "a")?;
    /// a.try_insert(2, "b")?;
    /// a.try_insert(3, "c")?;
    /// a.try_insert(4, "c")?;
    /// let cursor = a.lower_bound_mut(Bound::Excluded(&2));
    /// assert_eq!(cursor.key(), Some(&3));
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn lower_bound_mut<Q: ?Sized>(&mut self, bound: Bound<&Q>) -> CursorMut<'_, K, V, A>
    where
        K: Borrow<Q> + Ord,
        Q: Ord,
    {
        into_ok(self.lower_bound_mut_with(&mut (), bound, infallible_cmp))
    }

    pub(crate) fn lower_bound_mut_with<C: ?Sized, Q: ?Sized, E>(
        &mut self,
        cx: &mut C,
        bound: Bound<&Q>,
        cmp: CmpFn<C, Q, E>,
    ) -> Result<CursorMut<'_, K, V, A>, E>
    where
        K: Borrow<Q>,
    {
        let (root, dormant_root) = DormantMutRef::new(&mut self.root);

        let Some(root_node) = root.as_mut().map(NodeRef::borrow_mut) else {
            return Ok(CursorMut {
                current: None,
                root: dormant_root,
                length: &mut self.length,
                alloc: &mut *self.alloc,
            });
        };

        let edge = root_node.lower_bound(cx, SearchBound::from_range(bound), cmp)?;

        Ok(CursorMut {
            current: edge.next_kv().ok(),
            root: dormant_root,
            length: &mut self.length,
            alloc: &mut *self.alloc,
        })
    }

    /// Returns a [`Cursor`] pointing at the last element that is below the
    /// given bound.
    ///
    /// If no such element exists then a cursor pointing at the "ghost"
    /// non-element is returned.
    ///
    /// Passing [`Bound::Unbounded`] will return a cursor pointing at the last
    /// element of the map.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use rune::alloc::BTreeMap;
    /// use std::ops::Bound;
    ///
    /// let mut a = BTreeMap::new();
    /// a.try_insert(1, "a")?;
    /// a.try_insert(2, "b")?;
    /// a.try_insert(3, "c")?;
    /// a.try_insert(4, "c")?;
    /// let cursor = a.upper_bound(Bound::Excluded(&3));
    /// assert_eq!(cursor.key(), Some(&2));
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn upper_bound<Q>(&self, bound: Bound<&Q>) -> Cursor<'_, K, V>
    where
        K: Borrow<Q> + Ord,
        Q: Ord,
    {
        into_ok(self.upper_bound_with(&mut (), bound, infallible_cmp))
    }

    pub(crate) fn upper_bound_with<C: ?Sized, Q: ?Sized, E>(
        &self,
        cx: &mut C,
        bound: Bound<&Q>,
        cmp: CmpFn<C, Q, E>,
    ) -> Result<Cursor<'_, K, V>, E>
    where
        K: Borrow<Q>,
    {
        let Some(root_node) = self.root.as_ref().map(NodeRef::reborrow) else {
            return Ok(Cursor {
                current: None,
                root: None,
            });
        };

        let edge = root_node.upper_bound(cx, SearchBound::from_range(bound), cmp)?;

        Ok(Cursor {
            current: edge.next_back_kv().ok(),
            root: self.root.as_ref(),
        })
    }

    /// Returns a [`CursorMut`] pointing at the last element that is below the
    /// given bound.
    ///
    /// If no such element exists then a cursor pointing at the "ghost"
    /// non-element is returned.
    ///
    /// Passing [`Bound::Unbounded`] will return a cursor pointing at the last
    /// element of the map.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use rune::alloc::BTreeMap;
    /// use std::ops::Bound;
    ///
    /// let mut a = BTreeMap::new();
    /// a.try_insert(1, "a")?;
    /// a.try_insert(2, "b")?;
    /// a.try_insert(3, "c")?;
    /// a.try_insert(4, "c")?;
    /// let cursor = a.upper_bound_mut(Bound::Excluded(&3));
    /// assert_eq!(cursor.key(), Some(&2));
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn upper_bound_mut<Q: ?Sized>(&mut self, bound: Bound<&Q>) -> CursorMut<'_, K, V, A>
    where
        K: Borrow<Q>,
        Q: Ord,
    {
        into_ok(self.upper_bound_mut_with(&mut (), bound, infallible_cmp))
    }

    pub(crate) fn upper_bound_mut_with<C: ?Sized, Q: ?Sized, E>(
        &mut self,
        cx: &mut C,
        bound: Bound<&Q>,
        cmp: CmpFn<C, Q, E>,
    ) -> Result<CursorMut<'_, K, V, A>, E>
    where
        K: Borrow<Q>,
    {
        let (root, dormant_root) = DormantMutRef::new(&mut self.root);

        let Some(root_node) = root.as_mut().map(NodeRef::borrow_mut) else {
            return Ok(CursorMut {
                current: None,
                root: dormant_root,
                length: &mut self.length,
                alloc: &mut *self.alloc,
            });
        };

        let edge = root_node.upper_bound(cx, SearchBound::from_range(bound), cmp)?;

        Ok(CursorMut {
            current: edge.next_back_kv().ok(),
            root: dormant_root,
            length: &mut self.length,
            alloc: &mut *self.alloc,
        })
    }
}

/// A cursor over a `BTreeMap`.
///
/// A `Cursor` is like an iterator, except that it can freely seek back-and-forth.
///
/// Cursors always point to an element in the tree, and index in a logically circular way.
/// To accommodate this, there is a "ghost" non-element that yields `None` between the last and
/// first elements of the tree.
///
/// A `Cursor` is created with the [`BTreeMap::lower_bound`] and [`BTreeMap::upper_bound`] methods.
pub struct Cursor<'a, K: 'a, V: 'a> {
    current: Option<Handle<NodeRef<marker::Immut<'a>, K, V, marker::LeafOrInternal>, marker::KV>>,
    root: Option<&'a node::Root<K, V>>,
}

impl<K, V> Clone for Cursor<'_, K, V> {
    fn clone(&self) -> Self {
        let Cursor { current, root } = *self;
        Cursor { current, root }
    }
}

impl<K: Debug, V: Debug> Debug for Cursor<'_, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Cursor").field(&self.key_value()).finish()
    }
}

/// A cursor over a `BTreeMap` with editing operations.
///
/// A `Cursor` is like an iterator, except that it can freely seek back-and-forth, and can
/// safely mutate the tree during iteration. This is because the lifetime of its yielded
/// references is tied to its own lifetime, instead of just the underlying tree. This means
/// cursors cannot yield multiple elements at once.
///
/// Cursors always point to an element in the tree, and index in a logically circular way.
/// To accommodate this, there is a "ghost" non-element that yields `None` between the last and
/// first elements of the tree.
///
/// A `Cursor` is created with the [`BTreeMap::lower_bound_mut`] and [`BTreeMap::upper_bound_mut`]
/// methods.
pub struct CursorMut<'a, K: 'a, V: 'a, A = Global> {
    current: Option<Handle<NodeRef<marker::Mut<'a>, K, V, marker::LeafOrInternal>, marker::KV>>,
    root: DormantMutRef<'a, Option<node::Root<K, V>>>,
    length: &'a mut usize,
    alloc: &'a mut A,
}

impl<K: Debug, V: Debug, A> Debug for CursorMut<'_, K, V, A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("CursorMut").field(&self.key_value()).finish()
    }
}

impl<'a, K, V> Cursor<'a, K, V> {
    /// Moves the cursor to the next element of the `BTreeMap`.
    ///
    /// If the cursor is pointing to the "ghost" non-element then this will move it to
    /// the first element of the `BTreeMap`. If it is pointing to the last
    /// element of the `BTreeMap` then this will move it to the "ghost" non-element.
    pub(crate) fn move_next(&mut self) {
        match self.current.take() {
            None => {
                self.current = self.root.and_then(|root| {
                    root.reborrow()
                        .first_leaf_edge()
                        .forget_node_type()
                        .right_kv()
                        .ok()
                });
            }
            Some(current) => {
                self.current = current.next_leaf_edge().next_kv().ok();
            }
        }
    }

    /// Moves the cursor to the previous element of the `BTreeMap`.
    ///
    /// If the cursor is pointing to the "ghost" non-element then this will move it to
    /// the last element of the `BTreeMap`. If it is pointing to the first
    /// element of the `BTreeMap` then this will move it to the "ghost" non-element.
    pub(crate) fn move_prev(&mut self) {
        match self.current.take() {
            None => {
                self.current = self.root.and_then(|root| {
                    root.reborrow()
                        .last_leaf_edge()
                        .forget_node_type()
                        .left_kv()
                        .ok()
                });
            }
            Some(current) => {
                self.current = current.next_back_leaf_edge().next_back_kv().ok();
            }
        }
    }

    /// Returns a reference to the key of the element that the cursor is
    /// currently pointing to.
    ///
    /// This returns `None` if the cursor is currently pointing to the "ghost"
    /// non-element.
    pub fn key(&self) -> Option<&'a K> {
        self.current.as_ref().map(|current| current.into_kv().0)
    }

    /// Returns a reference to the value of the element that the cursor is
    /// currently pointing to.
    ///
    /// This returns `None` if the cursor is currently pointing to the "ghost"
    /// non-element.
    pub fn value(&self) -> Option<&'a V> {
        self.current.as_ref().map(|current| current.into_kv().1)
    }

    /// Returns a reference to the key and value of the element that the cursor
    /// is currently pointing to.
    ///
    /// This returns `None` if the cursor is currently pointing to the "ghost"
    /// non-element.
    pub fn key_value(&self) -> Option<(&'a K, &'a V)> {
        self.current.as_ref().map(|current| current.into_kv())
    }

    /// Returns a reference to the next element.
    ///
    /// If the cursor is pointing to the "ghost" non-element then this returns
    /// the first element of the `BTreeMap`. If it is pointing to the last
    /// element of the `BTreeMap` then this returns `None`.
    pub(crate) fn peek_next(&self) -> Option<(&'a K, &'a V)> {
        let mut next = self.clone();
        next.move_next();
        next.current.as_ref().map(|current| current.into_kv())
    }

    /// Returns a reference to the previous element.
    ///
    /// If the cursor is pointing to the "ghost" non-element then this returns
    /// the last element of the `BTreeMap`. If it is pointing to the first
    /// element of the `BTreeMap` then this returns `None`.
    pub(crate) fn peek_prev(&self) -> Option<(&'a K, &'a V)> {
        let mut prev = self.clone();
        prev.move_prev();
        prev.current.as_ref().map(|current| current.into_kv())
    }
}

impl<'a, K, V, A> CursorMut<'a, K, V, A> {
    /// Moves the cursor to the next element of the `BTreeMap`.
    ///
    /// If the cursor is pointing to the "ghost" non-element then this will move it to
    /// the first element of the `BTreeMap`. If it is pointing to the last
    /// element of the `BTreeMap` then this will move it to the "ghost" non-element.
    pub(crate) fn move_next(&mut self) {
        match self.current.take() {
            None => {
                // SAFETY: The previous borrow of root has ended.
                self.current = unsafe { self.root.reborrow() }.as_mut().and_then(|root| {
                    root.borrow_mut()
                        .first_leaf_edge()
                        .forget_node_type()
                        .right_kv()
                        .ok()
                });
            }
            Some(current) => {
                self.current = current.next_leaf_edge().next_kv().ok();
            }
        }
    }

    /// Moves the cursor to the previous element of the `BTreeMap`.
    ///
    /// If the cursor is pointing to the "ghost" non-element then this will move it to
    /// the last element of the `BTreeMap`. If it is pointing to the first
    /// element of the `BTreeMap` then this will move it to the "ghost" non-element.
    pub(crate) fn move_prev(&mut self) {
        match self.current.take() {
            None => {
                // SAFETY: The previous borrow of root has ended.
                self.current = unsafe { self.root.reborrow() }.as_mut().and_then(|root| {
                    root.borrow_mut()
                        .last_leaf_edge()
                        .forget_node_type()
                        .left_kv()
                        .ok()
                });
            }
            Some(current) => {
                self.current = current.next_back_leaf_edge().next_back_kv().ok();
            }
        }
    }

    /// Returns a reference to the key of the element that the cursor is
    /// currently pointing to.
    ///
    /// This returns `None` if the cursor is currently pointing to the "ghost"
    /// non-element.
    pub fn key(&self) -> Option<&K> {
        self.current
            .as_ref()
            .map(|current| current.reborrow().into_kv().0)
    }

    /// Returns a reference to the value of the element that the cursor is
    /// currently pointing to.
    ///
    /// This returns `None` if the cursor is currently pointing to the "ghost"
    /// non-element.
    pub fn value(&self) -> Option<&V> {
        self.current
            .as_ref()
            .map(|current| current.reborrow().into_kv().1)
    }

    /// Returns a reference to the key and value of the element that the cursor
    /// is currently pointing to.
    ///
    /// This returns `None` if the cursor is currently pointing to the "ghost"
    /// non-element.
    pub fn key_value(&self) -> Option<(&K, &V)> {
        self.current
            .as_ref()
            .map(|current| current.reborrow().into_kv())
    }

    /// Returns a mutable reference to the value of the element that the cursor
    /// is currently pointing to.
    ///
    /// This returns `None` if the cursor is currently pointing to the "ghost"
    /// non-element.
    pub fn value_mut(&mut self) -> Option<&mut V> {
        self.current.as_mut().map(|current| current.kv_mut().1)
    }

    /// Returns a reference to the key and mutable reference to the value of the
    /// element that the cursor is currently pointing to.
    ///
    /// This returns `None` if the cursor is currently pointing to the "ghost"
    /// non-element.
    pub fn key_value_mut(&mut self) -> Option<(&K, &mut V)> {
        self.current.as_mut().map(|current| {
            let (k, v) = current.kv_mut();
            (&*k, v)
        })
    }

    /// Returns a mutable reference to the key of the element that the cursor is
    /// currently pointing to.
    ///
    /// This returns `None` if the cursor is currently pointing to the
    /// "ghost" non-element.
    ///
    /// # Safety
    ///
    /// This can be used to modify the key, but you must ensure that the
    /// `BTreeMap` invariants are maintained. Specifically:
    ///
    /// * The key must remain unique within the tree.
    /// * The key must remain in sorted order with regards to other elements in
    ///   the tree.
    pub(crate) unsafe fn key_mut_unchecked(&mut self) -> Option<&mut K> {
        self.current.as_mut().map(|current| current.kv_mut().0)
    }

    /// Returns a reference to the key and value of the next element.
    ///
    /// If the cursor is pointing to the "ghost" non-element then this returns
    /// the first element of the `BTreeMap`. If it is pointing to the last
    /// element of the `BTreeMap` then this returns `None`.
    pub(crate) fn peek_next(&mut self) -> Option<(&K, &mut V)> {
        let (k, v) = match self.current {
            None => {
                // SAFETY: The previous borrow of root has ended.
                unsafe { self.root.reborrow() }
                    .as_mut()?
                    .borrow_mut()
                    .first_leaf_edge()
                    .next_kv()
                    .ok()?
                    .into_kv_valmut()
            }
            // SAFETY: We're not using this to mutate the tree.
            Some(ref mut current) => unsafe { current.reborrow_mut() }
                .next_leaf_edge()
                .next_kv()
                .ok()?
                .into_kv_valmut(),
        };
        Some((k, v))
    }

    /// Returns a reference to the key and value of the previous element.
    ///
    /// If the cursor is pointing to the "ghost" non-element then this returns
    /// the last element of the `BTreeMap`. If it is pointing to the first
    /// element of the `BTreeMap` then this returns `None`.
    pub(crate) fn peek_prev(&mut self) -> Option<(&K, &mut V)> {
        let (k, v) = match self.current.as_mut() {
            None => {
                // SAFETY: The previous borrow of root has ended.
                unsafe { self.root.reborrow() }
                    .as_mut()?
                    .borrow_mut()
                    .last_leaf_edge()
                    .next_back_kv()
                    .ok()?
                    .into_kv_valmut()
            }
            Some(current) => {
                // SAFETY: We're not using this to mutate the tree.
                unsafe { current.reborrow_mut() }
                    .next_back_leaf_edge()
                    .next_back_kv()
                    .ok()?
                    .into_kv_valmut()
            }
        };
        Some((k, v))
    }

    /// Returns a read-only cursor pointing to the current element.
    ///
    /// The lifetime of the returned `Cursor` is bound to that of the
    /// `CursorMut`, which means it cannot outlive the `CursorMut` and that the
    /// `CursorMut` is frozen for the lifetime of the `Cursor`.
    pub(crate) fn as_cursor(&self) -> Cursor<'_, K, V> {
        Cursor {
            // SAFETY: The tree is immutable while the cursor exists.
            root: unsafe { self.root.reborrow_shared().as_ref() },
            current: self.current.as_ref().map(|current| current.reborrow()),
        }
    }
}

// Now the tree editing operations
impl<'a, K: Ord, V, A: Allocator> CursorMut<'a, K, V, A> {
    /// Inserts a new element into the `BTreeMap` after the current one.
    ///
    /// If the cursor is pointing at the "ghost" non-element then the new element is
    /// inserted at the front of the `BTreeMap`.
    ///
    /// # Safety
    ///
    /// You must ensure that the `BTreeMap` invariants are maintained.
    /// Specifically:
    ///
    /// * The key of the newly inserted element must be unique in the tree.
    /// * All keys in the tree must remain in sorted order.
    pub(crate) unsafe fn try_insert_after_unchecked(
        &mut self,
        key: K,
        value: V,
    ) -> Result<(), AllocError> {
        let edge = match self.current.take() {
            None => {
                // SAFETY: We have no other reference to the tree.
                match unsafe { self.root.reborrow() } {
                    root @ None => {
                        // Tree is empty, allocate a new root.
                        let mut node = NodeRef::new_leaf(self.alloc)?;
                        node.borrow_mut().push(key, value);
                        *root = Some(node.forget_type());
                        *self.length += 1;
                        return Ok(());
                    }
                    Some(root) => root.borrow_mut().first_leaf_edge(),
                }
            }
            Some(current) => current.next_leaf_edge(),
        };

        let handle = edge.insert_recursing(key, value, self.alloc, |ins| {
            drop(ins.left);
            // SAFETY: The handle to the newly inserted value is always on a
            // leaf node, so adding a new root node doesn't invalidate it.
            let root = unsafe { self.root.reborrow().as_mut().unwrap() };
            root.push_internal_level(self.alloc)?
                .push(ins.kv.0, ins.kv.1, ins.right);
            Ok(())
        })?;
        self.current = handle.left_edge().next_back_kv().ok();
        *self.length += 1;
        Ok(())
    }

    /// Inserts a new element into the `BTreeMap` before the current one.
    ///
    /// If the cursor is pointing at the "ghost" non-element then the new element is
    /// inserted at the end of the `BTreeMap`.
    ///
    /// # Safety
    ///
    /// You must ensure that the `BTreeMap` invariants are maintained.
    /// Specifically:
    ///
    /// * The key of the newly inserted element must be unique in the tree.
    /// * All keys in the tree must remain in sorted order.
    pub(crate) unsafe fn try_insert_before_unchecked(
        &mut self,
        key: K,
        value: V,
    ) -> Result<(), AllocError> {
        let edge = match self.current.take() {
            None => {
                // SAFETY: We have no other reference to the tree.
                match unsafe { self.root.reborrow() } {
                    root @ None => {
                        // Tree is empty, allocate a new root.
                        let mut node = NodeRef::new_leaf(self.alloc)?;
                        node.borrow_mut().push(key, value);
                        *root = Some(node.forget_type());
                        *self.length += 1;
                        return Ok(());
                    }
                    Some(root) => root.borrow_mut().last_leaf_edge(),
                }
            }
            Some(current) => current.next_back_leaf_edge(),
        };

        let handle = edge.insert_recursing(key, value, self.alloc, |ins| {
            drop(ins.left);
            // SAFETY: The handle to the newly inserted value is always on a
            // leaf node, so adding a new root node doesn't invalidate it.
            let root = unsafe { self.root.reborrow().as_mut().unwrap() };
            root.push_internal_level(self.alloc)?
                .push(ins.kv.0, ins.kv.1, ins.right);
            Ok(())
        })?;
        self.current = handle.right_edge().next_kv().ok();
        *self.length += 1;
        Ok(())
    }

    /// Inserts a new element into the `BTreeMap` after the current one.
    ///
    /// If the cursor is pointing at the "ghost" non-element then the new element is
    /// inserted at the front of the `BTreeMap`.
    ///
    /// # Panics
    ///
    /// This function panics if:
    /// - the given key compares less than or equal to the current element (if
    ///   any).
    /// - the given key compares greater than or equal to the next element (if
    ///   any).
    pub(crate) fn try_insert_after(&mut self, key: K, value: V) -> Result<(), AllocError> {
        if let Some(current) = self.key() {
            if &key <= current {
                panic!("key must be ordered above the current element");
            }
        }
        if let Some((next, _)) = self.peek_next() {
            if &key >= next {
                panic!("key must be ordered below the next element");
            }
        }
        unsafe {
            self.try_insert_after_unchecked(key, value)?;
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn insert_after(&mut self, key: K, value: V) {
        self.try_insert_after(key, value).abort()
    }

    /// Inserts a new element into the `BTreeMap` before the current one.
    ///
    /// If the cursor is pointing at the "ghost" non-element then the new element is
    /// inserted at the end of the `BTreeMap`.
    ///
    /// # Panics
    ///
    /// This function panics if:
    /// - the given key compares greater than or equal to the current element
    ///   (if any).
    /// - the given key compares less than or equal to the previous element (if
    ///   any).
    pub(crate) fn try_insert_before(&mut self, key: K, value: V) -> Result<(), AllocError> {
        if let Some(current) = self.key() {
            if &key >= current {
                panic!("key must be ordered below the current element");
            }
        }
        if let Some((prev, _)) = self.peek_prev() {
            if &key <= prev {
                panic!("key must be ordered above the previous element");
            }
        }
        unsafe {
            self.try_insert_before_unchecked(key, value)?;
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn insert_before(&mut self, key: K, value: V) {
        self.try_insert_before(key, value).abort()
    }

    /// Removes the current element from the `BTreeMap`.
    ///
    /// The element that was removed is returned, and the cursor is
    /// moved to point to the next element in the `BTreeMap`.
    ///
    /// If the cursor is currently pointing to the "ghost" non-element then no element
    /// is removed and `None` is returned. The cursor is not moved in this case.
    pub(crate) fn remove_current(&mut self) -> Option<(K, V)> {
        let current = self.current.take()?;
        let mut emptied_internal_root = false;
        let (kv, pos) = current.remove_kv_tracking(|| emptied_internal_root = true, self.alloc);
        self.current = pos.next_kv().ok();
        *self.length -= 1;
        if emptied_internal_root {
            // SAFETY: This is safe since current does not point within the now
            // empty root node.
            let root = unsafe { self.root.reborrow().as_mut().unwrap() };
            root.pop_internal_level(self.alloc);
        }
        Some(kv)
    }

    /// Removes the current element from the `BTreeMap`.
    ///
    /// The element that was removed is returned, and the cursor is
    /// moved to point to the previous element in the `BTreeMap`.
    ///
    /// If the cursor is currently pointing to the "ghost" non-element then no element
    /// is removed and `None` is returned. The cursor is not moved in this case.
    pub(crate) fn remove_current_and_move_back(&mut self) -> Option<(K, V)> {
        let current = self.current.take()?;
        let mut emptied_internal_root = false;
        let (kv, pos) = current.remove_kv_tracking(|| emptied_internal_root = true, self.alloc);
        self.current = pos.next_back_kv().ok();
        *self.length -= 1;

        if emptied_internal_root {
            // SAFETY: This is safe since current does not point within the now
            // empty root node.
            let root = unsafe { self.root.reborrow().as_mut().unwrap() };
            root.pop_internal_level(self.alloc);
        }

        Some(kv)
    }
}

impl<K, V, A: Allocator> TryFromIteratorIn<(K, V), A> for BTreeMap<K, V, A>
where
    K: Ord,
{
    #[inline]
    fn try_from_iter_in<I>(iter: I, alloc: A) -> Result<Self, Error>
    where
        I: IntoIterator<Item = (K, V)>,
    {
        let mut this = BTreeMap::new_in(alloc);

        for (key, value) in iter {
            this.try_insert(key, value)?;
        }

        Ok(this)
    }
}

#[cfg(test)]
impl<K, V> FromIterator<(K, V)> for BTreeMap<K, V>
where
    K: Ord,
{
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
    {
        Self::try_from_iter_in(iter, Global).abort()
    }
}

impl<K, V, const N: usize> TryFrom<[(K, V); N]> for BTreeMap<K, V>
where
    K: Ord,
{
    type Error = Error;

    #[inline]
    fn try_from(values: [(K, V); N]) -> Result<Self, Self::Error> {
        let mut this = BTreeMap::new();

        for (key, value) in values {
            this.try_insert(key, value)?;
        }

        Ok(this)
    }
}

#[cfg(test)]
mod tests;
