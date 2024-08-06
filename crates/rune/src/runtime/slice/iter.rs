use crate as rune;
use crate::runtime::{Ref, Value};
use crate::Any;

/// An efficient reference counter iterator over a vector.
#[derive(Any)]
#[rune(item = ::std::slice)]
pub struct Iter {
    vec: Ref<[Value]>,
    front: usize,
    back: usize,
}

impl Iter {
    pub(crate) fn new(vec: Ref<[Value]>) -> Self {
        let back = vec.len();

        Self {
            vec,
            front: 0,
            back,
        }
    }

    #[rune::function(instance, keep, protocol = NEXT)]
    #[inline]
    fn next(&mut self) -> Option<Value> {
        if self.front == self.back {
            return None;
        }

        let value = self.vec.get(self.front)?;
        self.front = self.front.wrapping_add(1);
        Some(value.clone())
    }

    #[rune::function(instance, keep, protocol = NTH)]
    #[inline]
    fn nth(&mut self, n: usize) -> Option<Value> {
        let n = self.front.wrapping_add(n);

        if n >= self.back || n < self.front {
            return None;
        }

        let value = self.vec.get(n)?;
        self.front = n.wrapping_add(1);
        Some(value.clone())
    }

    #[rune::function(instance, keep, protocol = SIZE_HINT)]
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.back.wrapping_sub(self.front);
        (len, Some(len))
    }

    #[rune::function(instance, keep, protocol = LEN)]
    #[inline]
    fn len(&self) -> usize {
        self.back.wrapping_sub(self.front)
    }

    #[rune::function(instance, keep, protocol = NEXT_BACK)]
    #[inline]
    fn next_back(&mut self) -> Option<Value> {
        if self.front == self.back {
            return None;
        }

        self.back = self.back.wrapping_sub(1);
        let value = self.vec.get(self.back)?;
        Some(value.clone())
    }
}

impl Iterator for Iter {
    type Item = Value;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        Iter::next(self)
    }

    #[inline]
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        Iter::nth(self, n)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        Iter::size_hint(self)
    }
}

impl DoubleEndedIterator for Iter {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        Iter::next_back(self)
    }
}
