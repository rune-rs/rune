use core::fmt::{self, Write};
use core::iter;

use crate::no_std::collections;
use crate::no_std::prelude::*;

use crate::runtime::{Iterator, IteratorTrait, Key, Protocol, Ref, Value, VmResult};
use crate::{Any, ContextError, Module};

pub(super) fn setup(module: &mut Module) -> Result<(), ContextError> {
    module.ty::<HashSet>()?;
    module.function(["HashSet", "new"], HashSet::new)?;
    module.function(["HashSet", "from"], hashset_from)?;
    module.inst_fn("clear", HashSet::clear)?;
    module.inst_fn("clone", HashSet::clone)?;
    module.inst_fn("contains", HashSet::contains)?;
    module.inst_fn("difference", HashSet::difference)?;
    module.inst_fn("extend", HashSet::extend)?;
    module.inst_fn("insert", HashSet::insert)?;
    module.inst_fn("intersection", HashSet::intersection)?;
    module.inst_fn("is_empty", HashSet::is_empty)?;
    module.inst_fn("iter", HashSet::iter)?;
    module.inst_fn("len", HashSet::len)?;
    module.inst_fn("remove", HashSet::remove)?;
    module.inst_fn("union", HashSet::union)?;
    module.inst_fn(Protocol::INTO_ITER, HashSet::iter)?;
    module.inst_fn(Protocol::STRING_DEBUG, HashSet::string_debug)?;
    module.inst_fn(Protocol::EQ, HashSet::eq)?;
    Ok(())
}

#[derive(Any, Clone)]
#[rune(module = "crate")]
struct HashSet {
    set: collections::HashSet<Key>,
}

impl HashSet {
    fn new() -> Self {
        Self {
            set: collections::HashSet::new(),
        }
    }

    /// Extend this set from an iterator.
    #[inline]
    fn extend(&mut self, value: Value) -> VmResult<()> {
        let mut it = vm_try!(value.into_iter());

        while let Some(value) = vm_try!(it.next()) {
            let key = vm_try!(Key::from_value(&value));
            self.set.insert(key);
        }

        VmResult::Ok(())
    }

    #[inline]
    fn iter(&self) -> Iterator {
        let iter = self.set.clone().into_iter();
        Iterator::from("std::collections::set::Iter", iter)
    }

    #[inline]
    fn insert(&mut self, key: Key) -> bool {
        self.set.insert(key)
    }

    #[inline]
    fn contains(&self, key: Key) -> bool {
        self.set.contains(&key)
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.set.is_empty()
    }

    #[inline]
    fn len(&self) -> usize {
        self.set.len()
    }

    #[inline]
    fn clear(&mut self) {
        self.set.clear()
    }

    #[inline]
    fn remove(&mut self, key: Key) {
        self.set.remove(&key);
    }

    #[inline]
    fn difference(&self, other: Ref<HashSet>) -> Iterator {
        Iterator::from(
            "std::collections::set::Difference",
            Difference {
                this: self.set.clone().into_iter(),
                other: Some(other),
            },
        )
    }

    #[inline]
    fn intersection(zelf: Ref<HashSet>, other: Ref<HashSet>) -> Iterator {
        // use shortest iterator as driver for intersections
        let intersection = if zelf.len() <= other.len() {
            Intersection {
                this: zelf.set.clone().into_iter(),
                other: Some(other),
            }
        } else {
            Intersection {
                this: other.set.clone().into_iter(),
                other: Some(zelf),
            }
        };
        Iterator::from("std::collections::set::Intersection", intersection)
    }

    #[inline]
    fn union(zelf: Ref<HashSet>, other: Ref<HashSet>) -> VmResult<Iterator> {
        // use longest as lead and then append any missing that are in second
        let iter = Union {
            iter: if zelf.len() >= other.len() {
                vm_try!(zelf.iter().chain_raw(other.difference(zelf)))
            } else {
                vm_try!(other.iter().chain_raw(zelf.difference(other)))
            },
        };

        VmResult::Ok(Iterator::from("std::collections::set::Union", iter))
    }

    #[inline]
    fn string_debug(&self, s: &mut String) -> fmt::Result {
        write!(s, "{:?}", self.set)
    }

    #[inline]
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

fn hashset_from(value: Value) -> VmResult<HashSet> {
    let mut set = HashSet::new();
    let mut it = vm_try!(value.into_iter());

    while let Some(value) = vm_try!(it.next()) {
        set.insert(vm_try!(Key::from_value(&value)));
    }

    VmResult::Ok(set)
}
