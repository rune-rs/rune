use crate::runtime::{Ref, Value};

/// An efficient reference counter iterator over a vector.
pub(crate) struct Iter {
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
}

impl Iterator for Iter {
    type Item = Value;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.front == self.back {
            return None;
        }

        let value = self.vec.get(self.front)?;
        self.front = self.front.wrapping_add(1);
        Some(value.clone())
    }

    #[inline]
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        let n = self.front.wrapping_add(n);

        if n >= self.back || n < self.front {
            return None;
        }

        let value = self.vec.get(n)?;
        self.front = n.wrapping_add(1);
        Some(value.clone())
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.back.wrapping_sub(self.front);
        (len, Some(len))
    }
}

impl DoubleEndedIterator for Iter {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.front == self.back {
            return None;
        }

        self.back = self.back.wrapping_sub(1);
        let value = self.vec.get(self.back)?;
        Some(value.clone())
    }
}
