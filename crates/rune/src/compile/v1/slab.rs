use core::mem::replace;
use core::num::NonZeroUsize;

use crate::alloc::{self, Vec};

#[derive(Clone, Copy)]
enum Entry {
    Vacant(NonZeroUsize),
    Occupied,
}

pub(super) struct Slab {
    entries: Vec<Entry>,
    len: usize,
    next: usize,
}

impl Slab {
    pub(super) const fn new() -> Self {
        Self {
            entries: Vec::new(),
            len: 0,
            next: 0,
        }
    }

    pub(super) fn insert(&mut self) -> alloc::Result<Option<usize>> {
        let key = self.next;
        self.insert_at(key)?;
        Ok(Some(key))
    }

    pub(super) fn push(&mut self) -> alloc::Result<usize> {
        let key = self.entries.len();
        self.insert_at(key)?;
        Ok(key)
    }

    pub(super) fn len(&self) -> usize {
        self.len
    }

    pub(super) fn try_remove(&mut self, index: usize) -> bool {
        let Some(entry) = self.entries.get_mut(index) else {
            return false;
        };

        let Some(next) = to_index(self.next) else {
            return false;
        };

        let prev = replace(entry, Entry::Vacant(next));

        match prev {
            Entry::Occupied => {
                self.len -= 1;
                self.next = index;
                true
            }
            _ => {
                *entry = prev;
                false
            }
        }
    }

    fn insert_at(&mut self, key: usize) -> alloc::Result<()> {
        self.len += 1;

        if key == self.entries.len() {
            self.entries.try_push(Entry::Occupied)?;
            self.next = key + 1;
        } else {
            let next = match self.entries.get(key) {
                Some(&Entry::Vacant(next)) => from_index(next),
                _ => unreachable!(),
            };
            self.next = next;
            self.entries[key] = Entry::Occupied;
        }

        Ok(())
    }
}

#[inline]
fn to_index(index: usize) -> Option<NonZeroUsize> {
    NonZeroUsize::new(index ^ usize::MAX)
}

#[inline]
fn from_index(index: NonZeroUsize) -> usize {
    index.get() ^ usize::MAX
}
