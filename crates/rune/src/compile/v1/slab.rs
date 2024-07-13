use core::mem::replace;

use crate::alloc::{self, Vec};

enum Entry<T> {
    Vacant(usize),
    Occupied(T),
}

pub(super) struct Slab<T> {
    // Chunk of memory
    entries: Vec<Entry<T>>,

    // Number of Filled elements currently in the slab
    len: usize,

    // Offset of the next available slot in the slab. Set to the slab's
    // capacity when the slab is full.
    next: usize,
}

impl<T> Slab<T> {
    pub(super) const fn new() -> Self {
        Self {
            entries: Vec::new(),
            next: 0,
            len: 0,
        }
    }

    #[inline]
    #[allow(unused)]
    pub(super) fn len(&self) -> usize {
        self.len
    }

    pub(super) fn insert(&mut self, val: T) -> alloc::Result<usize> {
        let key = self.next;
        self.insert_at(key, val)?;
        Ok(key)
    }

    pub(super) fn try_remove(&mut self, key: usize) -> Option<T> {
        if let Some(entry) = self.entries.get_mut(key) {
            let prev = replace(entry, Entry::Vacant(self.next));

            match prev {
                Entry::Occupied(val) => {
                    self.len -= 1;
                    self.next = key;
                    return val.into();
                }
                _ => {
                    *entry = prev;
                }
            }
        }

        None
    }

    pub(super) fn get(&self, key: usize) -> Option<&T> {
        match self.entries.get(key) {
            Some(&Entry::Occupied(ref val)) => Some(val),
            _ => None,
        }
    }

    pub(super) fn get_mut(&mut self, key: usize) -> Option<&mut T> {
        match self.entries.get_mut(key) {
            Some(&mut Entry::Occupied(ref mut val)) => Some(val),
            _ => None,
        }
    }

    pub(super) fn vacant_key(&self) -> usize {
        self.next
    }

    fn insert_at(&mut self, key: usize, val: T) -> alloc::Result<()> {
        self.len += 1;

        if key == self.entries.len() {
            self.entries.try_push(Entry::Occupied(val))?;
            self.next = key + 1;
        } else {
            self.next = match self.entries.get(key) {
                Some(&Entry::Vacant(next)) => next,
                _ => unreachable!(),
            };
            self.entries[key] = Entry::Occupied(val);
        }

        Ok(())
    }
}
