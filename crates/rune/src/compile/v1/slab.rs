//! This is a specialized slab used to allocate slots of memory for the compiler.

use core::mem::replace;
use core::num::NonZeroUsize;

use crate::alloc::{self, Vec};

#[derive(Debug, Clone, Copy)]
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

    pub(super) fn insert(&mut self) -> alloc::Result<usize> {
        let key = self.next;
        self.insert_at(key)?;
        Ok(key)
    }

    pub(super) fn push(&mut self) -> alloc::Result<usize> {
        let key = self.entries.len();
        self.entries.try_push(Entry::Occupied)?;

        if key == self.next {
            self.next += 1;
        }

        self.len += 1;
        Ok(key)
    }

    pub(super) fn len(&self) -> usize {
        self.len
    }

    pub(super) fn remove(&mut self, index: usize) -> bool {
        // Remove tail entries so that pushing new entries always results in the
        // most compact linear slab possible.
        //
        // Note that the length is already correct.
        if index + 1 == self.entries.len() {
            return self.pop().is_some();
        }

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

    /// Pop the last element in the slab.
    pub(crate) fn pop(&mut self) -> Option<usize> {
        let next_is_last = self.next == self.entries.len();

        match self.entries.pop()? {
            Entry::Occupied => {
                self.len -= 1;

                if next_is_last {
                    self.next = self.entries.len();
                }
            }
            Entry::Vacant(last) => {
                debug_assert!(false, "This should not be possible");
                self.next = from_index(last);
            }
        }

        let index = self.entries.len();

        while let Some(Entry::Vacant(last)) = self.entries.last() {
            self.next = from_index(*last);
            self.entries.pop();
        }

        Some(index)
    }

    /// Insert a value at the given location.
    pub(crate) fn insert_at(&mut self, key: usize) -> alloc::Result<()> {
        self.len += 1;

        if key == self.entries.len() {
            self.entries.try_push(Entry::Occupied)?;
            self.next = key + 1;
        } else {
            let next = match self.entries.get(key) {
                Some(&Entry::Vacant(next)) => from_index(next),
                entry => unreachable!("{key} => {entry:?}"),
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

#[cfg(test)]
mod tests {
    use super::Slab;

    #[test]
    fn push() {
        let mut slab = Slab::new();
        assert_eq!(slab.insert(), Ok(0));
        assert_eq!(slab.push(), Ok(1));
        assert_eq!(slab.push(), Ok(2));
        assert_eq!(slab.len(), 3);

        assert_eq!(slab.remove(0), true);
        assert_eq!(slab.len(), 2);

        assert_eq!(slab.remove(0), false);
        assert_eq!(slab.len(), 2);

        assert_eq!(slab.insert(), Ok(0));
        assert_eq!(slab.len(), 3);

        assert_eq!(slab.insert(), Ok(3));
        assert_eq!(slab.len(), 4);

        assert_eq!(slab.remove(2), true);
        assert_eq!(slab.len(), 3);

        assert_eq!(slab.remove(0), true);
        assert_eq!(slab.len(), 2);

        assert_eq!(slab.insert(), Ok(0));
        assert_eq!(slab.len(), 3);

        assert_eq!(slab.insert(), Ok(2));
        assert_eq!(slab.len(), 4);

        assert_eq!(slab.insert(), Ok(4));
        assert_eq!(slab.len(), 5);

        assert_eq!(slab.push(), Ok(5));
        assert_eq!(slab.len(), 6);
    }

    #[test]
    fn push_tail_hole() {
        let mut slab = Slab::new();
        assert_eq!(slab.insert(), Ok(0));
        assert_eq!(slab.insert(), Ok(1));
        assert_eq!(slab.insert(), Ok(2));
        assert_eq!(slab.len(), 3);

        assert_eq!(slab.remove(1), true);
        assert_eq!(slab.remove(2), true);
        assert_eq!(slab.remove(2), false);
        assert_eq!(slab.len(), 1);

        assert_eq!(slab.push(), Ok(1));
        assert_eq!(slab.push(), Ok(2));

        assert_eq!(slab.len(), 3);
    }

    #[test]
    fn push_pop() {
        let mut slab = Slab::new();
        assert_eq!(slab.insert(), Ok(0));
        assert_eq!(slab.insert(), Ok(1));
        assert_eq!(slab.insert(), Ok(2));
        assert_eq!(slab.remove(1), true);
        assert_eq!(slab.len(), 2);

        assert_eq!(slab.push(), Ok(3));
        assert_eq!(slab.push(), Ok(4));
        assert_eq!(slab.push(), Ok(5));
        assert_eq!(slab.insert(), Ok(1));
        assert_eq!(slab.len(), 6);

        assert_eq!(slab.remove(2), true);
        assert_eq!(slab.len(), 5);

        assert_eq!(slab.pop(), Some(5));
        assert_eq!(slab.pop(), Some(4));
        assert_eq!(slab.pop(), Some(3));
        assert_eq!(slab.pop(), Some(1));
        assert_eq!(slab.pop(), Some(0));
        assert_eq!(slab.pop(), None);
        assert_eq!(slab.len(), 0);
    }
}
