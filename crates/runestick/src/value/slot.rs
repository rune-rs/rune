use crate::value::Generation;
use std::fmt;

/// Compact information on typed slot.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Slot(Generation, u32);

impl Slot {
    pub(crate) fn new(generation: usize, loc: usize) -> Self {
        Self(Generation(generation as u32), loc as u32)
    }

    /// Get the generation of the slot.
    #[inline]
    pub fn into_generation(self) -> usize {
        (self.0).0 as usize
    }

    /// Get the slot as an usize.
    #[inline]
    pub fn into_usize(self) -> usize {
        self.1 as usize
    }
}

impl fmt::Display for Slot {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "({}:{})", self.into_generation(), self.into_usize())
    }
}

#[cfg(test)]
mod tests {
    use super::Slot;

    #[test]
    fn test_slot() {
        assert_eq!(Slot::new(5, 77).into_generation(), 5);
        assert_eq!(Slot::new(5, 77).into_usize(), 77);
        assert_eq!(Slot::new(6, 78).into_generation(), 6);
        assert_eq!(Slot::new(5, 78).into_usize(), 78);
        assert_eq!(Slot::new(7, 79).into_generation(), 7);
        assert_eq!(Slot::new(7, 79).into_usize(), 79);
        assert_eq!(Slot::new(8, 80).into_generation(), 8);
        assert_eq!(Slot::new(8, 80).into_usize(), 80);
    }
}
