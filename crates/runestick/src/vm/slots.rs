//! A slab-like, pre-allocated storage where the slab is divided into immovable
//! slots. Each allocated slot doubles the capacity of the slab.
//!
//! Converted from <https://github.com/carllerche/slab>, this slab however
//! contains a growable collection of fixed-size regions called slots.
//! This allows is to store immovable objects inside the slab, since growing the
//! collection doesn't require the existing slots to move.

use crate::vm::Holder;
use std::{mem, ptr};

// Size of the first slot.
const FIRST_SLOT_SIZE: usize = 16;
// The initial number of bits to ignore for the first slot.
const FIRST_SLOT_MASK: usize =
    std::mem::size_of::<usize>() * 8 - FIRST_SLOT_SIZE.leading_zeros() as usize - 1;

/// Pre-allocated storage for a uniform data type, with slots of immovable
/// memory regions.
#[derive(Clone)]
pub struct Slots {
    // Slots of memory. Once one has been allocated it is never moved.
    // This allows us to store entries in there and fetch them as `Pin<&mut T>`.
    slots: Vec<ptr::NonNull<Entry<Holder>>>,
    // Number of Filled elements currently in the slab
    len: usize,
    // Offset of the next available slot in the slab.
    next: usize,
}

unsafe impl Send for Slots {}
unsafe impl Sync for Slots {}

enum Entry<T> {
    // Each slot is pre-allocated with entries of `None`.
    None,
    // Removed entries are replaced with the vacant tomb stone, pointing to the
    // next vacant entry.
    Vacant(usize),
    // An entry that is occupied with a value.
    Occupied(T),
}

impl Slots {
    pub(super) fn new() -> Self {
        Self {
            slots: Vec::new(),
            next: 0,
            len: 0,
        }
    }

    /// Insert a value into the pin slab.
    pub(super) fn insert(&mut self, val: Holder) -> usize {
        let key = self.next;
        self.insert_at(key, val);
        key
    }

    /// Get a reference to the value at the given slot.
    pub(super) fn get(&self, key: usize, generation: usize) -> Option<&Holder> {
        // Safety: We only use this to acquire an immutable reference.
        // The internal calculation guarantees that the key is in bounds.
        unsafe { self.internal_get(key, generation) }
    }

    /// Get a reference to the value at the given slot.
    #[inline(always)]
    unsafe fn internal_get(&self, key: usize, generation: usize) -> Option<&Holder> {
        let (slot, offset, len) = calculate_key(key);
        let slot = *self.slots.get(slot)?;

        // Safety: all slots are fully allocated and initialized in `new_slot`.
        // As long as we have access to it, we know that we will only find
        // initialized entries assuming offset < len.
        debug_assert!(offset < len);

        let entry = match &*slot.as_ptr().add(offset) {
            Entry::Occupied(entry) if entry.generation == generation => entry,
            _ => return None,
        };

        Some(entry)
    }

    /// Remove the key from the slab.
    ///
    /// Returns `true` if the entry was removed, `false` otherwise.
    /// Removing a key which does not exist has no effect, and `false` will be
    /// returned.
    ///
    /// We need to take care that we don't move it, hence we only perform
    /// operations over pointers below.
    pub(super) fn remove(&mut self, key: usize, generation: usize) -> Option<Holder> {
        let (slot, offset, len) = calculate_key(key);

        let slot = match self.slots.get_mut(slot) {
            Some(slot) => *slot,
            None => return None,
        };

        // Safety: all slots are fully allocated and initialized in `new_slot`.
        // As long as we have access to it, we know that we will only find
        // initialized entries assuming offset < len.
        debug_assert!(offset < len);
        unsafe {
            let entry = slot.as_ptr().add(offset);

            let holder = match ptr::replace(entry, Entry::Vacant(self.next)) {
                Entry::Occupied(holder) => {
                    if holder.generation == generation {
                        holder
                    } else {
                        *entry = Entry::Occupied(holder);
                        return None;
                    }
                }
                Entry::None => {
                    *entry = Entry::None;
                    return None;
                }
                Entry::Vacant(next) => {
                    *entry = Entry::Vacant(next);
                    return None;
                }
            };

            self.len -= 1;
            self.next = key;
            Some(holder)
        }
    }

    /// Clear all available data in the PinSlot.
    pub fn clear(&mut self) {
        for (len, entry) in slot_sizes().zip(self.slots.iter_mut()) {
            // reconstruct the vector for the slot.
            drop(unsafe { Vec::from_raw_parts(entry.as_ptr(), len, len) });
        }

        unsafe {
            self.slots.set_len(0);
        }
    }

    /// Construct a new slot.
    fn new_slot(&self, len: usize) -> ptr::NonNull<Entry<Holder>> {
        let mut d = Vec::with_capacity(len);

        for _ in 0..len {
            d.push(Entry::None);
        }

        let ptr = d.as_mut_ptr();
        mem::forget(d);

        // Safety: We just initialized the pointer to be non-null above.
        unsafe { ptr::NonNull::new_unchecked(ptr) }
    }

    /// Insert a value at the given slot.
    fn insert_at(&mut self, key: usize, holder: Holder) {
        let (slot, offset, len) = calculate_key(key);

        if let Some(slot) = self.slots.get_mut(slot) {
            // Safety: all slots are fully allocated and initialized in
            // `new_slot`. As long as we have access to it, we know that we will
            // only find initialized entries assuming offset < slot_size.
            // We also know it's safe to have unique access to _any_ slots,
            // since we have unique access to the slab in this function.
            debug_assert!(offset < len);
            let entry = unsafe { &mut *slot.as_ptr().add(offset) };

            self.next = match *entry {
                Entry::None => key + 1,
                Entry::Vacant(next) => next,
                // NB: unreachable because insert_at is an internal function,
                // which can only be appropriately called on non-occupied
                // entries. This is however, not a safety concern.
                _ => unreachable!(),
            };

            *entry = Entry::Occupied(holder);
        } else {
            unsafe {
                let slot = self.new_slot(len);
                *slot.as_ptr() = Entry::Occupied(holder);
                self.slots.push(slot);
                self.next = key + 1;
            }
        }

        self.len += 1;
    }
}

impl Drop for Slots {
    fn drop(&mut self) {
        self.clear();
    }
}

/// Calculate the key as a (slot, offset, len) tuple.
fn calculate_key(key: usize) -> (usize, usize, usize) {
    assert!(key < (1usize << (mem::size_of::<usize>() * 8 - 1)));

    let slot = ((mem::size_of::<usize>() * 8) as usize - key.leading_zeros() as usize)
        .saturating_sub(FIRST_SLOT_MASK);

    let (start, end) = if key < FIRST_SLOT_SIZE {
        (0, FIRST_SLOT_SIZE)
    } else {
        (FIRST_SLOT_SIZE << (slot - 1), FIRST_SLOT_SIZE << slot)
    };

    (slot, key - start, end - start)
}

fn slot_sizes() -> impl Iterator<Item = usize> {
    (0usize..).map(|n| match n {
        0 | 1 => FIRST_SLOT_SIZE,
        n => FIRST_SLOT_SIZE << (n - 1),
    })
}
