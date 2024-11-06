use core::ptr;

use crate as rune;
use crate::alloc::fmt::TryWrite;
use crate::alloc::hashbrown::raw::RawIter;
use crate::alloc::prelude::*;
use crate::hashbrown::{IterRef, Table};
use crate::runtime::{
    EnvProtocolCaller, Formatter, Iterator, ProtocolCaller, RawAnyGuard, Ref, Value, VmResult,
};
use crate::{Any, ContextError, Module};

/// A dynamic hash set.
#[rune::module(::std::collections::hash_set)]
pub fn module() -> Result<Module, ContextError> {
    let mut m = Module::from_meta(self::module_meta)?;

    m.ty::<HashSet>()?;
    m.function_meta(HashSet::new__meta)?;
    m.function_meta(HashSet::with_capacity__meta)?;
    m.function_meta(HashSet::len__meta)?;
    m.function_meta(HashSet::is_empty__meta)?;
    m.function_meta(HashSet::capacity__meta)?;
    m.function_meta(HashSet::insert__meta)?;
    m.function_meta(HashSet::remove__meta)?;
    m.function_meta(HashSet::contains__meta)?;
    m.function_meta(HashSet::clear__meta)?;
    m.function_meta(HashSet::difference__meta)?;
    m.function_meta(HashSet::extend__meta)?;
    m.function_meta(HashSet::intersection__meta)?;
    m.function_meta(HashSet::union__meta)?;
    m.function_meta(HashSet::iter__meta)?;
    m.function_meta(HashSet::into_iter__meta)?;
    m.function_meta(HashSet::from_iter__meta)?;
    m.function_meta(HashSet::debug_fmt__meta)?;

    m.function_meta(HashSet::clone__meta)?;
    m.implement_trait::<HashSet>(rune::item!(::std::clone::Clone))?;

    m.function_meta(HashSet::partial_eq__meta)?;
    m.implement_trait::<HashSet>(rune::item!(::std::cmp::PartialEq))?;

    m.function_meta(HashSet::eq__meta)?;
    m.implement_trait::<HashSet>(rune::item!(::std::cmp::Eq))?;

    m.ty::<Iter>()?;
    m.function_meta(Iter::next__meta)?;
    m.function_meta(Iter::size_hint__meta)?;
    m.implement_trait::<Iter>(rune::item!(::std::iter::Iterator))?;
    m.function_meta(Iter::len__meta)?;
    m.implement_trait::<Iter>(rune::item!(::std::iter::ExactSizeIterator))?;

    m.ty::<Difference>()?;
    m.function_meta(Difference::next__meta)?;
    m.function_meta(Difference::size_hint__meta)?;
    m.implement_trait::<Difference>(rune::item!(::std::iter::Iterator))?;

    m.ty::<Intersection>()?;
    m.function_meta(Intersection::next__meta)?;
    m.function_meta(Intersection::size_hint__meta)?;
    m.implement_trait::<Intersection>(rune::item!(::std::iter::Iterator))?;

    m.ty::<Union>()?;
    m.function_meta(Union::next__meta)?;
    m.implement_trait::<Union>(rune::item!(::std::iter::Iterator))?;
    Ok(m)
}

/// A [hash set] implemented as a `HashMap` where the value is `()`.
///
/// As with the [`HashMap`] type, a `HashSet` requires that the elements
/// implement the [`EQ`] and [`HASH`] protocols. If you implement these
/// yourself, it is important that the following property holds:
///
/// ```text
/// k1 == k2 -> hash(k1) == hash(k2)
/// ```
///
/// In other words, if two keys are equal, their hashes must be equal. Violating
/// this property is a logic error.
///
/// It is also a logic error for a key to be modified in such a way that the
/// key's hash, as determined by the [`HASH`] protocol, or its equality, as
/// determined by the [`EQ`] protocol, changes while it is in the map. This is
/// normally only possible through [`Cell`], [`RefCell`], global state, I/O, or
/// unsafe code.
///
/// The behavior resulting from either logic error is not specified, but will be
/// encapsulated to the `HashSet` that observed the logic error and not result
/// in undefined behavior. This could include panics, incorrect results, aborts,
/// memory leaks, and non-termination.
///
/// [hash set]: crate::collections#use-the-set-variant-of-any-of-these-maps-when
/// [`HashMap`]: crate::collections::HashMap
///
/// # Examples
///
/// ```rune
/// use std::collections::HashSet;
///
/// enum Tile {
///     Wall,
/// }
///
/// let m = HashSet::new();
///
/// m.insert((0, 1));
///
/// assert!(m.contains((0, 1)));
/// assert!(!m.contains((0, 2)));
/// ```
#[derive(Any)]
#[rune(module = crate, item = ::std::collections::hash_set)]
pub(crate) struct HashSet {
    table: Table<()>,
}

impl HashSet {
    /// Creates an empty `HashSet`.
    ///
    /// The hash set is initially created with a capacity of 0, so it will not
    /// allocate until it is first inserted into.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::collections::HashSet;
    ///
    /// let set = HashSet::new();
    /// ```
    #[rune::function(keep, path = Self::new)]
    fn new() -> Self {
        Self {
            table: Table::new(),
        }
    }

    /// Creates an empty `HashSet` with at least the specified capacity.
    ///
    /// The hash set will be able to hold at least `capacity` elements without
    /// reallocating. This method is allowed to allocate for more elements than
    /// `capacity`. If `capacity` is 0, the hash set will not allocate.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::collections::HashSet;
    ///
    /// let set = HashSet::with_capacity(10);
    /// assert!(set.capacity() >= 10);
    /// ```
    #[rune::function(keep, path = Self::with_capacity)]
    pub(crate) fn with_capacity(capacity: usize) -> VmResult<Self> {
        VmResult::Ok(Self {
            table: vm_try!(Table::try_with_capacity(capacity)),
        })
    }

    /// Returns the number of elements in the set.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::collections::HashSet;
    ///
    /// let v = HashSet::new();
    /// assert_eq!(v.len(), 0);
    /// v.insert(1);
    /// assert_eq!(v.len(), 1);
    /// ```
    #[rune::function(keep)]
    fn len(&self) -> usize {
        self.table.len()
    }

    /// Returns `true` if the set contains no elements.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::collections::HashSet;
    ///
    /// let v = HashSet::new();
    /// assert!(v.is_empty());
    /// v.insert(1);
    /// assert!(!v.is_empty());
    /// ```
    #[rune::function(keep)]
    fn is_empty(&self) -> bool {
        self.table.is_empty()
    }

    /// Returns the number of elements the set can hold without reallocating.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::collections::HashSet;
    ///
    /// let set = HashSet::with_capacity(100);
    /// assert!(set.capacity() >= 100);
    /// ```
    #[rune::function(keep)]
    fn capacity(&self) -> usize {
        self.table.capacity()
    }

    /// Adds a value to the set.
    ///
    /// Returns whether the value was newly inserted. That is:
    ///
    /// - If the set did not previously contain this value, `true` is returned.
    /// - If the set already contained this value, `false` is returned.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::collections::HashSet;
    ///
    /// let set = HashSet::new();
    ///
    /// assert_eq!(set.insert(2), true);
    /// assert_eq!(set.insert(2), false);
    /// assert_eq!(set.len(), 1);
    /// ```
    #[rune::function(keep)]
    pub(crate) fn insert(&mut self, key: Value) -> VmResult<bool> {
        let mut caller = EnvProtocolCaller;
        VmResult::Ok(vm_try!(self.table.insert_with(key, (), &mut caller)).is_none())
    }

    /// Removes a value from the set. Returns whether the value was present in
    /// the set.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::collections::HashSet;
    ///
    /// let set = HashSet::new();
    ///
    /// set.insert(2);
    /// assert_eq!(set.remove(2), true);
    /// assert_eq!(set.remove(2), false);
    /// ```
    #[rune::function(keep)]
    fn remove(&mut self, key: Value) -> VmResult<bool> {
        let mut caller = EnvProtocolCaller;
        VmResult::Ok(vm_try!(self.table.remove_with(&key, &mut caller)).is_some())
    }

    /// Returns `true` if the set contains a value.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::collections::HashSet;
    ///
    /// let set = HashSet::from_iter([1, 2, 3]);
    ///
    /// assert!(set.contains(1));
    /// assert!(!set.contains(4));
    /// ```
    #[rune::function(keep)]
    fn contains(&self, key: Value) -> VmResult<bool> {
        let mut caller = EnvProtocolCaller;
        VmResult::Ok(vm_try!(self.table.get(&key, &mut caller)).is_some())
    }

    /// Clears the set, removing all values.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::collections::HashSet;
    ///
    /// let v = HashSet::new();
    /// v.insert(1);
    /// v.clear();
    /// assert!(v.is_empty());
    /// ```
    #[rune::function(keep)]
    fn clear(&mut self) {
        self.table.clear()
    }

    /// Visits the values representing the difference, i.e., the values that are
    /// in `self` but not in `other`.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::collections::HashSet;
    ///
    /// let a = HashSet::from_iter([1, 2, 3]);
    /// let b = HashSet::from_iter([4, 2, 3, 4]);
    ///
    /// let diff = a.difference(b).collect::<HashSet>();
    /// assert_eq!(diff, [1].iter().collect::<HashSet>());
    ///
    /// // Note that difference is not symmetric,
    /// // and `b - a` means something else:
    /// let diff = b.difference(a).collect::<HashSet>();
    /// assert_eq!(diff, [4].iter().collect::<HashSet>());
    /// ```
    #[rune::function(keep, instance, path = Self::difference)]
    fn difference(this: Ref<Self>, other: Ref<Self>) -> Difference {
        Self::difference_inner(this, other)
    }

    fn difference_inner(this: Ref<Self>, other: Ref<Self>) -> Difference {
        Difference {
            this: Table::iter_ref(Ref::map(this, |this| &this.table)),
            other: Some(other),
        }
    }

    /// Visits the values representing the intersection, i.e., the values that
    /// are both in `self` and `other`.
    ///
    /// When an equal element is present in `self` and `other` then the
    /// resulting `Intersection` may yield values to one or the other.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::collections::HashSet;
    ///
    /// let a = HashSet::from_iter([1, 2, 3]);
    /// let b = HashSet::from_iter([4, 2, 3, 4]);
    ///
    /// let values = a.intersection(b).collect::<HashSet>();
    /// assert_eq!(values, [2, 3].iter().collect::<HashSet>());
    /// ```
    #[rune::function(keep, instance, path = Self::intersection)]
    fn intersection(this: Ref<Self>, other: Ref<Self>) -> Intersection {
        // use shortest iterator as driver for intersections
        if this.table.len() <= other.table.len() {
            Intersection {
                this: Table::iter_ref(Ref::map(this, |this| &this.table)),
                other: Some(other),
            }
        } else {
            Intersection {
                this: Table::iter_ref(Ref::map(other, |this| &this.table)),
                other: Some(this),
            }
        }
    }

    /// Visits the values representing the union, i.e., all the values in `self`
    /// or `other`, without duplicates.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::collections::HashSet;
    ///
    /// let a = HashSet::from_iter([1, 2, 3]);
    /// let b = HashSet::from_iter([4, 2, 3, 4]);
    ///
    /// let union = a.union(b).collect::<HashSet>();
    /// assert_eq!(union, HashSet::from_iter([1, 2, 3, 4]));
    ///
    /// let union = b.union(a).collect::<HashSet>();
    /// assert_eq!(union, HashSet::from_iter([1, 2, 3, 4]));
    /// ```
    #[rune::function(keep, instance, path = Self::union)]
    fn union(this: Ref<Self>, other: Ref<Self>) -> VmResult<Union> {
        unsafe {
            let (this, this_guard) = Ref::into_raw(Ref::map(this, |this| &this.table));
            let (other, other_guard) = Ref::into_raw(Ref::map(other, |this| &this.table));

            // use longest as lead and then append any missing that are in second
            let iter = if this.as_ref().len() >= other.as_ref().len() {
                let this_iter = Table::iter_ref_raw(this);
                let other_iter = Table::iter_ref_raw(other);

                Union {
                    this,
                    this_iter,
                    other_iter,
                    _guards: (this_guard, other_guard),
                }
            } else {
                let this_iter = Table::iter_ref_raw(other);
                let other_iter = Table::iter_ref_raw(this);

                Union {
                    this: other,
                    this_iter,
                    other_iter,
                    _guards: (other_guard, this_guard),
                }
            };

            VmResult::Ok(iter)
        }
    }

    /// Iterate over the hash set.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::collections::HashSet;
    ///
    /// let set = HashSet::from_iter([3, 2, 1]);
    /// let vec = set.iter().collect::<Vec>();
    /// vec.sort();
    /// assert_eq!(vec, [1, 2, 3]);
    /// ```
    #[rune::function(keep, instance, path = Self::iter)]
    fn iter(this: Ref<Self>) -> Iter {
        let iter = Table::iter_ref(Ref::map(this, |this| &this.table));

        Iter { iter }
    }

    /// Extend this set from an iterator.
    #[rune::function(keep)]
    fn extend(&mut self, value: Value) -> VmResult<()> {
        let mut caller = EnvProtocolCaller;
        let mut it = vm_try!(value.into_iter());

        while let Some(key) = vm_try!(it.next()) {
            vm_try!(self.table.insert_with(key, (), &mut caller));
        }

        VmResult::Ok(())
    }

    /// Convert the set into an iterator.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::collections::HashSet;
    ///
    /// let set = HashSet::from_iter([3, 2, 1]);
    /// let vec = [];
    ///
    /// for value in set {
    ///     vec.push(value);
    /// }
    ///
    /// vec.sort();
    /// assert_eq!(vec, [1, 2, 3]);
    /// ```
    #[rune::function(keep, instance, protocol = INTO_ITER, path = Self)]
    fn into_iter(this: Ref<Self>) -> Iter {
        Self::iter(this)
    }

    /// Write a debug representation to a string.
    ///
    /// This calls the [`DEBUG_FMT`] protocol over all elements of the
    /// collection.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::collections::HashSet;
    ///
    /// let set = HashSet::from_iter([1, 2, 3]);
    /// println!("{:?}", set);
    /// ```
    #[rune::function(keep, protocol = DEBUG_FMT)]
    fn debug_fmt(&self, f: &mut Formatter) -> VmResult<()> {
        self.debug_fmt_with(f, &mut EnvProtocolCaller)
    }

    fn debug_fmt_with(&self, f: &mut Formatter, _: &mut dyn ProtocolCaller) -> VmResult<()> {
        vm_try!(vm_write!(f, "{{"));

        let mut it = self.table.iter().peekable();

        while let Some(value) = it.next() {
            vm_try!(vm_write!(f, "{:?}", value));

            if it.peek().is_some() {
                vm_try!(vm_write!(f, ", "));
            }
        }

        vm_try!(vm_write!(f, "}}"));
        VmResult::Ok(())
    }

    /// Convert a [`HashSet`] from an iterator.
    ///
    /// The hashset can be converted from anything that implements the
    /// [`INTO_ITER`] protocol.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::collections::HashSet;
    ///
    /// let set = HashSet::from_iter(["a", "b"]);
    /// assert_eq!(set.len(), 2);
    /// assert!(set.contains("a"));
    /// assert!(set.contains("b"));
    /// ```
    #[rune::function(keep, path = Self::from_iter)]
    fn from_iter(it: Iterator) -> VmResult<HashSet> {
        let mut caller = EnvProtocolCaller;
        Self::from_iter_with(it, &mut caller)
    }

    pub(crate) fn from_iter_with(
        mut it: Iterator,
        caller: &mut dyn ProtocolCaller,
    ) -> VmResult<Self> {
        let (lo, _) = vm_try!(it.size_hint());
        let mut set = vm_try!(Table::try_with_capacity(lo));

        while let Some(key) = vm_try!(it.next()) {
            vm_try!(set.insert_with(key, (), caller));
        }

        VmResult::Ok(HashSet { table: set })
    }

    /// Perform a partial equality test between two sets.
    ///
    /// # Examples
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::collections::HashSet;
    ///
    /// let set = HashSet::from_iter([1, 2, 3]);
    /// assert_eq!(set, HashSet::from_iter([1, 2, 3]));
    /// assert_ne!(set, HashSet::from_iter([2, 3, 4]));
    /// ```
    #[rune::function(keep, protocol = PARTIAL_EQ)]
    fn partial_eq(&self, other: &Self) -> VmResult<bool> {
        self.eq_with(other, &mut EnvProtocolCaller)
    }

    /// Perform a total equality test between two sets.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::ops::eq;
    /// use std::collections::HashSet;
    ///
    /// let set = HashSet::from_iter([1, 2, 3]);
    /// assert!(eq(set, HashSet::from_iter([1, 2, 3])));
    /// assert!(!eq(set, HashSet::from_iter([2, 3, 4])));
    /// ```
    #[rune::function(keep, protocol = EQ)]
    fn eq(&self, other: &Self) -> VmResult<bool> {
        self.eq_with(other, &mut EnvProtocolCaller)
    }

    fn eq_with(&self, other: &Self, caller: &mut EnvProtocolCaller) -> VmResult<bool> {
        if self.table.len() != other.table.len() {
            return VmResult::Ok(false);
        }

        for (key, ()) in self.table.iter() {
            if vm_try!(other.table.get(key, caller)).is_none() {
                return VmResult::Ok(false);
            }
        }

        VmResult::Ok(true)
    }

    #[rune::function(keep, instance, path = Self::clone, protocol = CLONE)]
    fn clone(this: &HashSet) -> VmResult<HashSet> {
        VmResult::Ok(Self {
            table: vm_try!(this.table.try_clone()),
        })
    }
}

#[derive(Any)]
#[rune(item = ::std::collections::hash_set)]
struct Iter {
    iter: IterRef<()>,
}

impl Iter {
    #[rune::function(keep, instance, protocol = NEXT)]
    pub(crate) fn next(&mut self) -> Option<Value> {
        let (value, ()) = self.iter.next()?;
        Some(value)
    }

    #[rune::function(keep, instance, protocol = SIZE_HINT)]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }

    #[rune::function(keep, instance, protocol = LEN)]
    fn len(&self) -> usize {
        self.iter.len()
    }
}

#[derive(Any)]
#[rune(item = ::std::collections::hash_set)]
struct Intersection {
    this: IterRef<()>,
    other: Option<Ref<HashSet>>,
}

impl Intersection {
    #[rune::function(keep, instance, protocol = NEXT)]
    pub(crate) fn next(&mut self) -> VmResult<Option<Value>> {
        let mut caller = EnvProtocolCaller;

        let Some(other) = &self.other else {
            return VmResult::Ok(None);
        };

        for (key, ()) in self.this.by_ref() {
            let c = vm_try!(other.table.get(&key, &mut caller)).is_some();

            if c {
                return VmResult::Ok(Some(key));
            }
        }

        self.other = None;
        VmResult::Ok(None)
    }

    #[rune::function(keep, instance, protocol = SIZE_HINT)]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (_, upper) = self.this.size_hint();
        (0, upper)
    }
}

#[derive(Any)]
#[rune(item = ::std::collections::hash_set)]
struct Difference {
    this: IterRef<()>,
    other: Option<Ref<HashSet>>,
}

impl Difference {
    #[rune::function(keep, instance, protocol = NEXT)]
    pub(crate) fn next(&mut self) -> VmResult<Option<Value>> {
        let mut caller = EnvProtocolCaller;

        let Some(other) = &self.other else {
            return VmResult::Ok(None);
        };

        for (key, ()) in self.this.by_ref() {
            let c = vm_try!(other.table.get(&key, &mut caller)).is_some();

            if !c {
                return VmResult::Ok(Some(key));
            }
        }

        self.other = None;
        VmResult::Ok(None)
    }

    #[rune::function(keep, instance, protocol = SIZE_HINT)]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (_, upper) = self.this.size_hint();
        (0, upper)
    }
}

#[derive(Any)]
#[rune(item = ::std::collections::hash_set)]
struct Union {
    this: ptr::NonNull<Table<()>>,
    this_iter: RawIter<(Value, ())>,
    other_iter: RawIter<(Value, ())>,
    _guards: (RawAnyGuard, RawAnyGuard),
}

impl Union {
    #[rune::function(keep, instance, protocol = NEXT)]
    fn next(&mut self) -> VmResult<Option<Value>> {
        // SAFETY: we're holding onto the ref guards for both collections during
        // iteration, so this is valid for the lifetime of the iterator.
        unsafe {
            if let Some(bucket) = self.this_iter.next() {
                let (value, ()) = bucket.as_ref();
                return VmResult::Ok(Some(value.clone()));
            }

            let mut caller = EnvProtocolCaller;

            for bucket in self.other_iter.by_ref() {
                let (key, ()) = bucket.as_ref();

                if vm_try!(self.this.as_ref().get(key, &mut caller)).is_none() {
                    return VmResult::Ok(Some(key.clone()));
                }
            }

            VmResult::Ok(None)
        }
    }
}
