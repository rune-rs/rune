use crate::alloc::fmt::TryWrite;
use core::iter;
use core::ptr;

use crate as rune;

use crate::alloc::hashbrown::raw::RawIter;
use crate::alloc::prelude::*;
use crate::hashbrown::{IterRef, Table};
use crate::runtime::{
    EnvProtocolCaller, Formatter, Iterator, ProtocolCaller, RawRef, Ref, Value, VmResult,
};
use crate::{Any, ContextError, Module};

pub(super) fn setup(module: &mut Module) -> Result<(), ContextError> {
    module.ty::<HashSet>()?;
    module.function_meta(HashSet::new__meta)?;
    module.function_meta(HashSet::with_capacity__meta)?;
    module.function_meta(HashSet::len__meta)?;
    module.function_meta(HashSet::is_empty__meta)?;
    module.function_meta(HashSet::capacity__meta)?;
    module.function_meta(HashSet::insert__meta)?;
    module.function_meta(HashSet::remove__meta)?;
    module.function_meta(HashSet::contains__meta)?;
    module.function_meta(HashSet::clear__meta)?;
    module.function_meta(HashSet::difference__meta)?;
    module.function_meta(HashSet::extend__meta)?;
    module.function_meta(HashSet::intersection__meta)?;
    module.function_meta(HashSet::union__meta)?;
    module.function_meta(HashSet::iter__meta)?;
    module.function_meta(HashSet::into_iter__meta)?;
    module.function_meta(HashSet::string_debug__meta)?;
    module.function_meta(HashSet::partial_eq__meta)?;
    module.function_meta(HashSet::eq__meta)?;
    module.function_meta(HashSet::clone__meta)?;
    module.function_meta(HashSet::from__meta)?;
    Ok(())
}

#[derive(Any)]
#[rune(module = crate, item = ::std::collections)]
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
    fn with_capacity(capacity: usize) -> VmResult<Self> {
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
    fn insert(&mut self, key: Value) -> VmResult<bool> {
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
    /// let set = HashSet::from([1, 2, 3]);
    /// assert_eq!(set.contains(1), true);
    /// assert_eq!(set.contains(4), false);
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
    #[rune::function(keep, instance, path = Self::difference)]
    fn difference(this: Ref<Self>, other: Ref<Self>) -> Iterator {
        let iter = Self::difference_inner(this, other);
        Iterator::from("std::collections::hash_set::Difference", iter)
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
    /// let a = HashSet::from([1, 2, 3]);
    /// let b = HashSet::from([4, 2, 3, 4]);
    ///
    /// let values = a.intersection(b).collect::<HashSet>();
    /// assert_eq!(values, [2, 3].iter().collect::<HashSet>());
    /// ```
    #[rune::function(keep, instance, path = Self::intersection)]
    fn intersection(this: Ref<Self>, other: Ref<Self>) -> Iterator {
        // use shortest iterator as driver for intersections
        let iter = if this.table.len() <= other.table.len() {
            Intersection {
                this: Table::iter_ref(Ref::map(this, |this| &this.table)),
                other: Some(other),
            }
        } else {
            Intersection {
                this: Table::iter_ref(Ref::map(other, |this| &this.table)),
                other: Some(this),
            }
        };

        Iterator::from("std::collections::hash_set::Intersection", iter)
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
    ///
    /// let union = b.union(a).collect::<HashSet>();
    /// assert_eq!(union, HashSet::from([1, 2, 3, 4]));
    /// ```
    #[rune::function(keep, instance, path = Self::union)]
    fn union(this: Ref<Self>, other: Ref<Self>) -> VmResult<Iterator> {
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

            VmResult::Ok(Iterator::from("std::collections::hash_set::Union", iter))
        }
    }

    /// Iterate over the hash set.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::collections::HashSet;
    ///
    /// let set = HashSet::from([3, 2, 1]);
    /// let vec = set.iter().collect::<Vec>();
    /// vec.sort();
    /// assert_eq!(vec, [1, 2, 3]);
    /// ```
    #[rune::function(keep, instance, path = Self::iter)]
    fn iter(this: Ref<Self>) -> VmResult<Iterator> {
        let iter = Self::iter_inner(this);
        VmResult::Ok(Iterator::from("std::collections::hash_set::Iter", iter))
    }

    fn iter_inner(this: Ref<Self>) -> impl iter::Iterator<Item = Value> {
        Table::iter_ref(Ref::map(this, |this| &this.table)).map(|(key, ())| key)
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
    /// let set = HashSet::from([3, 2, 1]);
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
    fn into_iter(this: Ref<Self>) -> VmResult<Iterator> {
        Self::iter(this)
    }

    /// Write a debug representation to a string.
    ///
    /// This calls the [`STRING_DEBUG`] protocol over all elements of the
    /// collection.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::collections::HashSet;
    ///
    /// let set = HashSet::from([1, 2, 3]);
    /// println!("{:?}", set);
    /// ```
    #[rune::function(keep, protocol = STRING_DEBUG)]
    fn string_debug(&self, f: &mut Formatter) -> VmResult<()> {
        self.string_debug_with(f, &mut EnvProtocolCaller)
    }

    fn string_debug_with(&self, f: &mut Formatter, _: &mut impl ProtocolCaller) -> VmResult<()> {
        vm_write!(f, "{{");

        let mut it = self.table.iter().peekable();

        while let Some(value) = it.next() {
            vm_write!(f, "{:?}", value);

            if it.peek().is_some() {
                vm_write!(f, ", ");
            }
        }

        vm_write!(f, "}}");
        VmResult::Ok(())
    }

    pub(crate) fn from_iter<P>(mut it: Iterator, caller: &mut P) -> VmResult<Self>
    where
        P: ?Sized + ProtocolCaller,
    {
        let mut set = vm_try!(Table::try_with_capacity(it.size_hint().0));

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
    /// let set = HashSet::from([1, 2, 3]);
    /// assert_eq!(set, HashSet::from([1, 2, 3]));
    /// assert_ne!(set, HashSet::from([2, 3, 4]));
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
    /// let set = HashSet::from([1, 2, 3]);
    /// assert!(eq(set, HashSet::from([1, 2, 3])));
    /// assert!(!eq(set, HashSet::from([2, 3, 4])));
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

    #[rune::function(keep, path = Self::from)]
    fn from(value: Value) -> VmResult<HashSet> {
        let mut caller = EnvProtocolCaller;
        HashSet::from_iter(vm_try!(value.into_iter()), &mut caller)
    }

    #[rune::function(keep, instance, path = Self::clone)]
    fn clone(this: &HashSet) -> VmResult<HashSet> {
        VmResult::Ok(Self {
            table: vm_try!(this.table.try_clone()),
        })
    }
}

struct Intersection {
    this: IterRef<()>,
    other: Option<Ref<HashSet>>,
}

impl iter::Iterator for Intersection {
    type Item = VmResult<Value>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut caller = EnvProtocolCaller;
        let other = self.other.as_ref()?;

        for (key, ()) in self.this.by_ref() {
            let c = match other.table.get(&key, &mut caller) {
                VmResult::Ok(c) => c.is_some(),
                VmResult::Err(e) => return Some(VmResult::Err(e)),
            };

            if c {
                return Some(VmResult::Ok(key));
            }
        }

        self.other = None;
        None
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (_, upper) = self.this.size_hint();
        (0, upper)
    }
}

struct Difference {
    this: IterRef<()>,
    other: Option<Ref<HashSet>>,
}

impl iter::Iterator for Difference {
    type Item = VmResult<Value>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut caller = EnvProtocolCaller;
        let other = self.other.as_ref()?;

        for (key, ()) in self.this.by_ref() {
            let c = match other.table.get(&key, &mut caller) {
                VmResult::Ok(c) => c.is_some(),
                VmResult::Err(e) => return Some(VmResult::Err(e)),
            };

            if !c {
                return Some(VmResult::Ok(key));
            }
        }

        self.other = None;
        None
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (_, upper) = self.this.size_hint();
        (0, upper)
    }
}

struct Union {
    this: ptr::NonNull<Table<()>>,
    this_iter: RawIter<(Value, ())>,
    other_iter: RawIter<(Value, ())>,
    _guards: (RawRef, RawRef),
}

impl iter::Iterator for Union {
    type Item = VmResult<Value>;

    fn next(&mut self) -> Option<Self::Item> {
        // SAFETY: we're holding onto the ref guards for both collections during
        // iteration, so this is valid for the lifetime of the iterator.
        unsafe {
            if let Some(bucket) = self.this_iter.next() {
                let (value, ()) = bucket.as_ref();
                return Some(VmResult::Ok(value.clone()));
            }

            let mut caller = EnvProtocolCaller;

            for bucket in self.other_iter.by_ref() {
                let (key, ()) = bucket.as_ref();

                match self.this.as_ref().get(key, &mut caller) {
                    VmResult::Ok(None) => return Some(VmResult::Ok(key.clone())),
                    VmResult::Ok(..) => {}
                    VmResult::Err(e) => return Some(VmResult::Err(e)),
                }
            }

            None
        }
    }
}
