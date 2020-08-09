//! A slab storage for slot data. Specialized to understand generational
//! storage.
//!
//! Largely copied from <https://github.com/carllerche/slab>, under the MIT
//! license. Original copyright notice follows.
//!
//! ```text
//! Copyright (c) 2019 Carl Lerche
//!
//! Permission is hereby granted, free of charge, to any
//! person obtaining a copy of this software and associated
//! documentation files (the "Software"), to deal in the
//! Software without restriction, including without
//! limitation the rights to use, copy, modify, merge,
//! publish, distribute, sublicense, and/or sell copies of
//! the Software, and to permit persons to whom the Software
//! is furnished to do so, subject to the following
//! conditions:
//!
//! The above copyright notice and this permission notice
//! shall be included in all copies or substantial portions
//! of the Software.
//!
//! THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
//! ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
//! TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
//! PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
//! SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
//! CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
//! OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
//! IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
//! DEALINGS IN THE SOFTWARE.
//! ```

use crate::vm::Holder;
use std::mem;

/// Pre-allocated storage for a uniform data type, with slots of immovable
/// memory regions.
pub struct Slots {
    // Chunk of memory
    entries: Vec<Entry<Holder>>,
    // Number of Filled elements currently in the slab
    len: usize,
    // Offset of the next available slot in the slab.
    next: usize,
}

unsafe impl Send for Slots {}
unsafe impl Sync for Slots {}

enum Entry<T> {
    // Removed entries are replaced with the vacant tomb stone, pointing to the
    // next vacant entry.
    Vacant(usize),
    // An entry that is occupied with a value.
    Occupied(T),
}

impl Slots {
    pub(super) fn new() -> Self {
        Self {
            entries: Vec::new(),
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
        match self.entries.get(key) {
            Some(Entry::Occupied(holder)) if holder.generation == generation => Some(holder),
            _ => None,
        }
    }

    /// Remove the holder from the slab of values, given that its generation
    /// matches.
    ///
    /// Returns the removed holder, or `None` if it was not present.
    pub(super) fn remove(&mut self, key: usize, generation: usize) -> Option<Holder> {
        // Swap the entry at the provided value
        let prev = mem::replace(&mut self.entries[key], Entry::Vacant(self.next));

        match prev {
            Entry::Occupied(holder) => {
                if holder.generation == generation {
                    self.len -= 1;
                    self.next = key;
                    Some(holder)
                } else {
                    self.entries[key] = Entry::Occupied(holder);
                    None
                }
            }
            _ => {
                self.entries[key] = prev;
                None
            }
        }
    }

    /// Clear all stored values.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.len = 0;
        self.next = 0;
    }

    /// Insert a value at the given slot.
    fn insert_at(&mut self, key: usize, holder: Holder) {
        self.len += 1;

        if key == self.entries.len() {
            self.entries.push(Entry::Occupied(holder));
            self.next = key + 1;
        } else {
            let prev = mem::replace(&mut self.entries[key], Entry::Occupied(holder));

            match prev {
                Entry::Vacant(next) => {
                    self.next = next;
                }
                _ => unreachable!(),
            }
        }
    }
}
