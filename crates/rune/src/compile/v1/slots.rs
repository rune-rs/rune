//! This is a specialized slab used to allocate slots of memory for the compiler.

#![allow(clippy::bool_assert_comparison)]

#[cfg(test)]
mod tests;

use core::fmt;
use core::slice;

use crate::alloc::{self, Vec};

pub(super) struct Slots {
    storage: Vec<u128>,
    head: usize,
}

impl Slots {
    /// Construct a new empty slab.
    pub(super) const fn new() -> Self {
        Self {
            storage: Vec::new(),
            head: 0,
        }
    }

    /// Allocate the first free variable.
    #[tracing::instrument(ret(level = tracing::Level::TRACE), skip(self))]
    pub(super) fn insert(&mut self) -> alloc::Result<usize> {
        let mut key = (u128::BITS as usize) * self.head;

        for bits in self
            .storage
            .get_mut(self.head..)
            .unwrap_or_default()
            .iter_mut()
        {
            if *bits == u128::MAX {
                key += u128::BITS as usize;
                self.head += 1;
                continue;
            }

            let o = bits.trailing_ones();
            key += o as usize;
            *bits |= 1 << o;
            return Ok(key);
        }

        self.head = self.storage.len();
        self.storage.try_push(1)?;
        Ok(key)
    }

    #[tracing::instrument(ret(level = tracing::Level::TRACE), skip(self))]
    pub(super) fn push(&mut self) -> alloc::Result<usize> {
        let mut last = None;

        let key = 'key: {
            for (n, bits) in self.storage.iter_mut().enumerate().rev() {
                let o = bits.leading_zeros();

                // Whole segment is free, skip over it.
                if o == u128::BITS {
                    last = Some((n, bits));
                    continue;
                }

                let key = (u128::BITS as usize) * n;

                // There is no space in this segment.
                if o == 0 {
                    break 'key key + u128::BITS as usize;
                }

                let o = u128::BITS - o;
                *bits |= 1 << o;
                return Ok(key + o as usize);
            }

            0
        };

        if let Some((n, bits)) = last {
            *bits = 1;
            self.storage.truncate(n + 1);
        } else {
            self.storage.try_push(1)?;
        }

        Ok(key)
    }

    #[tracing::instrument(ret(level = tracing::Level::TRACE), skip(self))]
    pub(super) fn remove(&mut self, key: usize) -> bool {
        let index = key / (u128::BITS as usize);

        let Some(bits) = self.storage.get_mut(index) else {
            return false;
        };

        self.head = index;
        let o = key % (u128::BITS as usize);
        let existed = *bits & (1 << o) != 0;
        *bits &= !(1 << o);
        existed
    }

    fn iter(&self) -> Iter<'_> {
        let (current, rest) = match &self.storage[..] {
            [first, rest @ ..] => (*first, rest),
            [] => (0, &[][..]),
        };

        Iter {
            storage: rest.iter(),
            current,
            key: 0,
        }
    }
}

impl fmt::Debug for Slots {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

struct Iter<'a> {
    storage: slice::Iter<'a, u128>,
    current: u128,
    key: usize,
}

impl Iterator for Iter<'_> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let o = self.current.trailing_zeros();

            if o != u128::BITS {
                self.current ^= 1 << o;
                return Some(self.key + o as usize);
            }

            self.current = match self.storage.next() {
                Some(current) => *current,
                None => return None,
            };

            self.key += u128::BITS as usize;
        }
    }
}
