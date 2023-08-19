use core::fmt::{self, Write};
use core::iter;

use crate as rune;
use crate::no_std::collections;
use crate::no_std::prelude::*;

use crate::runtime::{Iterator, IteratorTrait, Key, Protocol, Ref, Value, VmResult};
use crate::{Any, ContextError, Module};

pub(super) fn setup(module: &mut Module) -> Result<(), ContextError> {
    module.ty::<HashSet>()?;
    module.function_meta(HashSet::new)?;
    module.function_meta(HashSet::with_capacity)?;
    module.function_meta(HashSet::len)?;
    module.function_meta(HashSet::is_empty)?;
    module.function_meta(HashSet::capacity)?;
    module.function_meta(HashSet::insert)?;
    module.function_meta(HashSet::remove)?;
    module.function_meta(HashSet::contains)?;
    module.function_meta(HashSet::clear)?;
    module.function_meta(HashSet::difference)?;
    module.function_meta(HashSet::extend)?;
    module.function_meta(HashSet::intersection)?;
    module.function_meta(HashSet::union)?;
    module.function_meta(HashSet::iter)?;
    module.function_meta(clone)?;
    module.function_meta(from)?;
    module.associated_function(Protocol::INTO_ITER, HashSet::__rune_fn__iter)?;
    module.associated_function(Protocol::STRING_DEBUG, HashSet::string_debug)?;
    module.associated_function(Protocol::EQ, HashSet::eq)?;
    Ok(())
}

#[derive(Any, Clone)]
#[rune(module = crate, item = ::std::collections)]
pub(crate) struct HashSet {
    set: collections::HashSet<Key>,
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
    #[rune::function(path = Self::new)]
    fn new() -> Self {
        Self {
            set: collections::HashSet::new(),
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
    #[rune::function(path = Self::with_capacity)]
    fn with_capacity(capacity: usize) -> Self {
        Self {
            set: collections::HashSet::with_capacity(capacity),
        }
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
    #[rune::function]
    fn len(&self) -> usize {
        self.set.len()
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
    #[rune::function]
    fn is_empty(&self) -> bool {
        self.set.is_empty()
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
    #[rune::function]
    fn capacity(&self) -> usize {
        self.set.capacity()
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
    #[rune::function]
    fn insert(&mut self, key: Key) -> bool {
        self.set.insert(key)
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
    #[rune::function]
    fn remove(&mut self, key: Key) -> bool {
        self.set.remove(&key)
    }

    /// Returns `true` if the set contains a value.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::collections::HashSet;
    ///
    /// let set = HashSet::from([1, 2, 3]);
    /// assert_eq!(set.contains(1), true);
    /// assert_eq!(set.contains(4), false);
    /// ```
    #[rune::function]
    fn contains(&self, key: Key) -> bool {
        self.set.contains(&key)
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
    #[rune::function]
    fn clear(&mut self) {
        self.set.clear()
    }

    /// Visits the values representing the difference, i.e., the values that are
    /// in `self` but not in `other`.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::collections::HashSet;
    ///
    /// let a = HashSet::from([1, 2, 3]);
    /// let b = HashSet::from([4, 2, 3, 4]);
    ///
    /// let diff = a.difference(b).collect::<HashSet>();
    /// assert_eq!(diff, [1].iter().collect::<HashSet>());
    ///
    /// // Note that difference is not symmetric,
    /// // and `b - a` means something else:
    /// let diff = b.difference(a).collect::<HashSet>();
    /// assert_eq!(diff, [4].iter().collect::<HashSet>());
    /// ```
    #[rune::function(instance, path = Self::difference)]
    fn difference(this: Ref<Self>, other: Ref<Self>) -> Iterator {
        Iterator::from(
            "std::collections::set::Difference",
            Difference {
                this: this.set.clone().into_iter(),
                other: Some(other),
            },
        )
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
    /// let a = HashSet::from([1, 2, 3]);
    /// let b = HashSet::from([4, 2, 3, 4]);
    ///
    /// let values = a.intersection(b).collect::<HashSet>();
    /// assert_eq!(values, [2, 3].iter().collect::<HashSet>());
    /// ```
    #[rune::function(instance, path = Self::intersection)]
    fn intersection(this: Ref<Self>, other: Ref<Self>) -> Iterator {
        // use shortest iterator as driver for intersections
        let intersection = if this.set.len() <= other.set.len() {
            Intersection {
                this: this.set.clone().into_iter(),
                other: Some(other),
            }
        } else {
            Intersection {
                this: other.set.clone().into_iter(),
                other: Some(this),
            }
        };

        Iterator::from("std::collections::set::Intersection", intersection)
    }

    /// Visits the values representing the union, i.e., all the values in `self`
    /// or `other`, without duplicates.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::collections::HashSet;
    ///
    /// let a = HashSet::from([1, 2, 3]);
    /// let b = HashSet::from([4, 2, 3, 4]);
    ///
    /// let union = a.union(b).collect::<HashSet>();
    /// assert_eq!(union, HashSet::from([1, 2, 3, 4]));
    /// ```
    #[rune::function(instance, path = Self::union)]
    fn union(this: Ref<Self>, other: Ref<Self>) -> VmResult<Iterator> {
        // use longest as lead and then append any missing that are in second
        let iter = Union {
            iter: if this.set.len() >= other.set.len() {
                vm_try!(HashSet::__rune_fn__iter(&this)
                    .chain_raw(HashSet::__rune_fn__difference(other, this)))
            } else {
                vm_try!(HashSet::__rune_fn__iter(&other)
                    .chain_raw(HashSet::__rune_fn__difference(this, other)))
            },
        };

        VmResult::Ok(Iterator::from("std::collections::set::Union", iter))
    }

    #[rune::function]
    fn iter(&self) -> Iterator {
        let iter = self.set.clone().into_iter();
        Iterator::from("std::collections::set::Iter", iter)
    }

    /// Extend this set from an iterator.
    #[rune::function]
    fn extend(&mut self, value: Value) -> VmResult<()> {
        let mut it = vm_try!(value.into_iter());

        while let Some(value) = vm_try!(it.next()) {
            let key = vm_try!(Key::from_value(&value));
            self.set.insert(key);
        }

        VmResult::Ok(())
    }

    #[inline]
    fn string_debug(&self, s: &mut String) -> fmt::Result {
        write!(s, "{:?}", self.set)
    }

    pub(crate) fn from_iter(mut it: Iterator) -> VmResult<Self> {
        let mut set = collections::HashSet::with_capacity(it.size_hint().0);

        while let Some(value) = vm_try!(it.next()) {
            set.insert(vm_try!(Key::from_value(&value)));
        }

        VmResult::Ok(HashSet { set })
    }

    fn eq(&self, other: &Self) -> bool {
        self.set == other.set
    }
}

struct Intersection<I>
where
    I: iter::Iterator<Item = Key>,
{
    this: I,
    other: Option<Ref<HashSet>>,
}

impl<I> iter::Iterator for Intersection<I>
where
    I: iter::Iterator<Item = Key>,
{
    type Item = Key;
    fn next(&mut self) -> Option<Self::Item> {
        let other = self.other.take()?;

        loop {
            let item = self.this.next()?;

            if other.set.contains(&item) {
                self.other = Some(other);
                return Some(item);
            }
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (_, upper) = self.this.size_hint();
        (0, upper)
    }
}

struct Difference<I>
where
    I: iter::Iterator<Item = Key>,
{
    this: I,
    other: Option<Ref<HashSet>>,
}

impl<I> iter::Iterator for Difference<I>
where
    I: iter::Iterator<Item = Key>,
{
    type Item = Key;

    fn next(&mut self) -> Option<Self::Item> {
        let other = self.other.take()?;

        loop {
            let item = self.this.next()?;

            if !other.set.contains(&item) {
                self.other = Some(other);
                return Some(item);
            }
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (_, upper) = self.this.size_hint();
        (0, upper)
    }
}

struct Union {
    iter: Iterator,
}

impl IteratorTrait for Union {
    fn next(&mut self) -> VmResult<Option<Value>> {
        self.iter.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

#[rune::function(path = HashSet::from)]
fn from(value: Value) -> VmResult<HashSet> {
    HashSet::from_iter(vm_try!(value.into_iter()))
}

#[rune::function(instance)]
fn clone(this: &HashSet) -> HashSet {
    this.clone()
}
