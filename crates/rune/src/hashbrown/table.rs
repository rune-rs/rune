use rune_alloc::hash_map;

use core::hash::BuildHasher;
use core::iter;
use core::marker::PhantomData;
use core::mem;
use core::ptr;

use rune_alloc::hashbrown::raw::{RawIter, RawTable};
use rune_alloc::hashbrown::ErrorOrInsertSlot;

use crate::alloc;
use crate::alloc::prelude::*;
use crate::runtime::{Hasher, ProtocolCaller, RawAnyGuard, Ref, Value, VmError};

pub(crate) struct Table<V> {
    table: RawTable<(Value, V)>,
    state: hash_map::RandomState,
}

impl<V> Table<V> {
    #[inline(always)]
    pub(crate) fn new() -> Self {
        Self {
            table: RawTable::new(),
            state: hash_map::RandomState::new(),
        }
    }

    #[inline(always)]
    pub(crate) fn try_with_capacity(capacity: usize) -> alloc::Result<Self> {
        Ok(Self {
            table: RawTable::try_with_capacity(capacity)?,
            state: hash_map::RandomState::new(),
        })
    }

    #[inline(always)]
    pub(crate) fn len(&self) -> usize {
        self.table.len()
    }

    #[inline(always)]
    pub(crate) fn capacity(&self) -> usize {
        self.table.capacity()
    }

    #[inline(always)]
    pub(crate) fn is_empty(&self) -> bool {
        self.table.is_empty()
    }

    #[inline(always)]
    pub(crate) fn insert_with(
        &mut self,
        key: Value,
        value: V,
        caller: &mut dyn ProtocolCaller,
    ) -> Result<Option<V>, VmError> {
        let hash = hash(&self.state, &key, caller)?;

        let existing = match self.table.find_or_find_insert_slot(
            caller,
            hash,
            KeyEq::new(&key),
            StateHasher::new(&self.state),
        ) {
            Ok(bucket) => Some(mem::replace(unsafe { &mut bucket.as_mut().1 }, value)),
            Err(ErrorOrInsertSlot::InsertSlot(slot)) => {
                unsafe {
                    self.table.insert_in_slot(hash, slot, (key, value));
                }
                None
            }
            Err(ErrorOrInsertSlot::Error(error)) => return Err(VmError::from(error)),
        };

        Ok(existing)
    }

    pub(crate) fn get(
        &self,
        key: &Value,
        caller: &mut dyn ProtocolCaller,
    ) -> Result<Option<&(Value, V)>, VmError> {
        if self.table.is_empty() {
            return Ok(None);
        }

        let hash = hash(&self.state, key, caller)?;
        self.table.get(caller, hash, KeyEq::new(key))
    }

    #[inline(always)]
    pub(crate) fn remove_with(
        &mut self,
        key: &Value,
        caller: &mut dyn ProtocolCaller,
    ) -> Result<Option<V>, VmError> {
        let hash = hash(&self.state, key, caller)?;

        match self.table.remove_entry(caller, hash, KeyEq::new(key)) {
            Ok(value) => Ok(value.map(|(_, value)| value)),
            Err(error) => Err(error),
        }
    }

    #[inline(always)]
    pub(crate) fn clear(&mut self) {
        self.table.clear()
    }

    pub(crate) fn iter(&self) -> Iter<'_, V> {
        // SAFETY: lifetime is held by returned iterator.
        let iter = unsafe { self.table.iter() };

        Iter {
            iter,
            _marker: PhantomData,
        }
    }

    #[inline(always)]
    pub(crate) fn iter_ref(this: Ref<Self>) -> IterRef<V> {
        let (this, _guard) = Ref::into_raw(this);
        // SAFETY: Table will be alive and a reference to it held for as long as
        // `RawAnyGuard` is alive.
        let iter = unsafe { this.as_ref().table.iter() };
        IterRef {
            iter,
            guard: _guard,
        }
    }

    #[inline(always)]
    pub(crate) unsafe fn iter_ref_raw(this: ptr::NonNull<Table<V>>) -> RawIter<(Value, V)> {
        this.as_ref().table.iter()
    }

    #[inline(always)]
    pub(crate) fn keys_ref(this: Ref<Self>) -> KeysRef<V> {
        let (this, _guard) = Ref::into_raw(this);
        // SAFETY: Table will be alive and a reference to it held for as long as
        // `RawAnyGuard` is alive.
        let iter = unsafe { this.as_ref().table.iter() };
        KeysRef {
            iter,
            guard: _guard,
        }
    }

    #[inline(always)]
    pub(crate) fn values_ref(this: Ref<Self>) -> ValuesRef<V> {
        let (this, _guard) = Ref::into_raw(this);
        // SAFETY: Table will be alive and a reference to it held for as long as
        // `RawAnyGuard` is alive.
        let iter = unsafe { this.as_ref().table.iter() };
        ValuesRef {
            iter,
            guard: _guard,
        }
    }
}

impl<V> TryClone for Table<V>
where
    V: TryClone,
{
    fn try_clone(&self) -> alloc::Result<Self> {
        Ok(Self {
            table: self.table.try_clone()?,
            state: self.state.clone(),
        })
    }

    #[inline]
    fn try_clone_from(&mut self, source: &Self) -> alloc::Result<()> {
        self.table.try_clone_from(&source.table)
    }
}

pub(crate) struct Iter<'a, V> {
    iter: RawIter<(Value, V)>,
    _marker: PhantomData<&'a V>,
}

impl<'a, V> iter::Iterator for Iter<'a, V> {
    type Item = &'a (Value, V);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        // SAFETY: we're still holding onto the `RawAnyGuard` guard.
        unsafe { Some(self.iter.next()?.as_ref()) }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

pub(crate) struct IterRef<V> {
    iter: RawIter<(Value, V)>,
    #[allow(unused)]
    guard: RawAnyGuard,
}

impl<V> iter::Iterator for IterRef<V>
where
    V: Clone,
{
    type Item = (Value, V);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        // SAFETY: we're still holding onto the `RawAnyGuard` guard.
        unsafe { Some(self.iter.next()?.as_ref().clone()) }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<V> iter::ExactSizeIterator for IterRef<V>
where
    V: Clone,
{
    #[inline]
    fn len(&self) -> usize {
        self.iter.len()
    }
}

pub(crate) struct KeysRef<V> {
    iter: RawIter<(Value, V)>,
    #[allow(unused)]
    guard: RawAnyGuard,
}

impl<V> iter::Iterator for KeysRef<V> {
    type Item = Value;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        // SAFETY: we're still holding onto the `RawAnyGuard` guard.
        unsafe { Some(self.iter.next()?.as_ref().0.clone()) }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

pub(crate) struct ValuesRef<V> {
    iter: RawIter<(Value, V)>,
    #[allow(unused)]
    guard: RawAnyGuard,
}

impl<V> iter::Iterator for ValuesRef<V>
where
    V: Clone,
{
    type Item = V;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        // SAFETY: we're still holding onto the `RawAnyGuard` guard.
        unsafe { Some(self.iter.next()?.as_ref().1.clone()) }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

/// Convenience function to hash a value.
fn hash<S>(state: &S, value: &Value, caller: &mut dyn ProtocolCaller) -> Result<u64, VmError>
where
    S: BuildHasher<Hasher = hash_map::Hasher>,
{
    let mut hasher = Hasher::new_with(state);
    value.hash_with(&mut hasher, caller)?;
    Ok(hasher.finish())
}

struct StateHasher<'a> {
    state: &'a hash_map::RandomState,
}

impl<'a> StateHasher<'a> {
    #[inline]
    fn new(state: &'a hash_map::RandomState) -> Self {
        Self { state }
    }
}

impl<V> rune_alloc::hashbrown::HasherFn<dyn ProtocolCaller, (Value, V), VmError>
    for StateHasher<'_>
{
    #[inline]
    fn hash(&self, cx: &mut dyn ProtocolCaller, (key, _): &(Value, V)) -> Result<u64, VmError> {
        hash(self.state, key, cx)
    }
}

/// Construct an equality function for a value in the table that will compare an
/// entry with the current key.
struct KeyEq<'a> {
    key: &'a Value,
}

impl<'a> KeyEq<'a> {
    #[inline]
    fn new(key: &'a Value) -> Self {
        Self { key }
    }
}

impl<V> rune_alloc::hashbrown::EqFn<dyn ProtocolCaller, (Value, V), VmError> for KeyEq<'_> {
    #[inline]
    fn eq(&self, cx: &mut dyn ProtocolCaller, (other, _): &(Value, V)) -> Result<bool, VmError> {
        self.key.eq_with(other, cx)
    }
}
