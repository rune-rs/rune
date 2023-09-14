use core::hash::BuildHasher;
use core::iter;
use core::marker::PhantomData;
use core::mem;
use core::ptr;

use crate::hashbrown::fork::raw::{RawIter, RawTable};
use std::collections::hash_map::{DefaultHasher, RandomState};

use crate::runtime::{Hasher, ProtocolCaller, RawRef, Ref, Value, VmError, VmResult};

#[derive(Clone)]
pub(crate) struct Table<V> {
    table: RawTable<(Value, V)>,
    state: RandomState,
}

impl<V> Table<V> {
    #[inline(always)]
    pub(crate) fn new() -> Self {
        Self {
            table: RawTable::new(),
            state: RandomState::new(),
        }
    }

    #[inline(always)]
    pub(crate) fn with_capacity(capacity: usize) -> Self {
        Self {
            table: RawTable::with_capacity(capacity),
            state: RandomState::new(),
        }
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
    pub(crate) fn insert_with<P>(
        &mut self,
        key: Value,
        value: V,
        caller: &mut P,
    ) -> VmResult<Option<V>>
    where
        P: ?Sized + ProtocolCaller,
    {
        let hash = vm_try!(hash(&self.state, &key, caller));

        let result = match self.table.find_or_find_insert_slot_with(
            caller,
            hash,
            eq(&key),
            hasher(&self.state),
        ) {
            Ok(result) => result,
            Err(error) => return VmResult::Err(error),
        };

        let existing = match result {
            Ok(bucket) => Some(mem::replace(unsafe { &mut bucket.as_mut().1 }, value)),
            Err(slot) => {
                unsafe {
                    self.table.insert_in_slot(hash, slot, (key, value));
                }
                None
            }
        };

        VmResult::Ok(existing)
    }

    pub(crate) fn get<P>(&self, key: &Value, caller: &mut P) -> VmResult<Option<&(Value, V)>>
    where
        P: ?Sized + ProtocolCaller,
    {
        if self.table.is_empty() {
            return VmResult::Ok(None);
        }

        let hash = vm_try!(hash(&self.state, key, caller));
        VmResult::Ok(vm_try!(self.table.get_with(caller, hash, eq(key))))
    }

    #[inline(always)]
    pub(crate) fn remove_with<P>(&mut self, key: &Value, caller: &mut P) -> VmResult<Option<V>>
    where
        P: ?Sized + ProtocolCaller,
    {
        let hash = vm_try!(hash(&self.state, key, caller));

        match self.table.remove_entry_with(caller, hash, eq(key)) {
            Ok(value) => VmResult::Ok(value.map(|(_, value)| value)),
            Err(error) => VmResult::Err(error),
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
        // `RawRef` is alive.
        let iter = unsafe { this.as_ref().table.iter() };
        IterRef { iter, _guard }
    }

    #[inline(always)]
    pub(crate) unsafe fn iter_ref_raw(this: ptr::NonNull<Table<V>>) -> RawIter<(Value, V)> {
        this.as_ref().table.iter()
    }

    #[inline(always)]
    pub(crate) fn keys_ref(this: Ref<Self>) -> KeysRef<V> {
        let (this, _guard) = Ref::into_raw(this);
        // SAFETY: Table will be alive and a reference to it held for as long as
        // `RawRef` is alive.
        let iter = unsafe { this.as_ref().table.iter() };
        KeysRef { iter, _guard }
    }

    #[inline(always)]
    pub(crate) fn values_ref(this: Ref<Self>) -> ValuesRef<V> {
        let (this, _guard) = Ref::into_raw(this);
        // SAFETY: Table will be alive and a reference to it held for as long as
        // `RawRef` is alive.
        let iter = unsafe { this.as_ref().table.iter() };
        ValuesRef { iter, _guard }
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
        // SAFETY: we're still holding onto the `RawRef` guard.
        unsafe { Some(self.iter.next()?.as_ref()) }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

pub(crate) struct IterRef<V> {
    iter: RawIter<(Value, V)>,
    _guard: RawRef,
}

impl<V> iter::Iterator for IterRef<V>
where
    V: Clone,
{
    type Item = (Value, V);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        // SAFETY: we're still holding onto the `RawRef` guard.
        unsafe { Some(self.iter.next()?.as_ref().clone()) }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

pub(crate) struct KeysRef<V> {
    iter: RawIter<(Value, V)>,
    _guard: RawRef,
}

impl<V> iter::Iterator for KeysRef<V> {
    type Item = Value;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        // SAFETY: we're still holding onto the `RawRef` guard.
        unsafe { Some(self.iter.next()?.as_ref().0.clone()) }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

pub(crate) struct ValuesRef<V> {
    iter: RawIter<(Value, V)>,
    _guard: RawRef,
}

impl<V> iter::Iterator for ValuesRef<V>
where
    V: Clone,
{
    type Item = V;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        // SAFETY: we're still holding onto the `RawRef` guard.
        unsafe { Some(self.iter.next()?.as_ref().1.clone()) }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

/// Convenience function to hash a value.
fn hash<S>(state: &S, value: &Value, caller: &mut impl ProtocolCaller) -> VmResult<u64>
where
    S: BuildHasher<Hasher = DefaultHasher>,
{
    let mut hasher = Hasher::new_with(state);
    vm_try!(value.hash_with(&mut hasher, caller));
    VmResult::Ok(hasher.finish())
}

/// Construct a hasher for a value in the table.
fn hasher<P, V, S>(state: &S) -> impl Fn(&mut P, &(Value, V)) -> Result<u64, VmError> + '_
where
    P: ?Sized + ProtocolCaller,
    S: BuildHasher<Hasher = DefaultHasher>,
{
    move |caller, (key, _): &(Value, V)| hash(state, key, caller).into_result()
}

/// Construct an equality function for a value in the table that will compare an
/// entry with the current key.
fn eq<P, V>(key: &Value) -> impl Fn(&mut P, &(Value, V)) -> Result<bool, VmError> + '_
where
    P: ?Sized + ProtocolCaller,
{
    move |caller: &mut P, (other, _): &(Value, V)| -> Result<bool, VmError> {
        key.eq_with(other, caller).into_result()
    }
}
