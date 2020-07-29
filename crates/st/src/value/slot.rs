use crate::value::{Managed, ValuePtr};
use crate::vm::StackError;
use std::fmt;

/// Compact information on typed slot.
#[derive(Clone, Copy)]
pub struct Slot(usize);

impl Slot {
    const STRING: usize = 0;
    const ARRAY: usize = 1;
    const EXTERNAL: usize = 2;

    /// Slot
    pub fn into_managed(self) -> (Managed, usize) {
        let slot = (self.0 >> 2) as usize;

        match self.0 & 0b11 {
            0 => (Managed::String, slot),
            1 => (Managed::Array, slot),
            _ => (Managed::External, slot),
        }
    }

    /// Construct a string slot.
    pub fn string(slot: usize) -> Self {
        Self(slot << 2 | Self::STRING)
    }

    /// Construct an array slot.
    pub fn array(slot: usize) -> Self {
        Self(slot << 2 | Self::ARRAY)
    }

    /// Construct an external slot.
    pub fn external(slot: usize) -> Self {
        Self(slot << 2 | Self::EXTERNAL)
    }
}

impl fmt::Debug for Slot {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (managed, slot) = self.into_managed();
        write!(fmt, "{}({})", managed, slot)
    }
}

macro_rules! decl_managed {
    ($name:ident, $constant:ident) => {
        #[allow(unused)]
        pub(super) struct $name(());

        impl IntoSlot for $name {
            fn into_slot(value: ValuePtr) -> Result<usize, StackError> {
                let Slot(slot) = match value {
                    ValuePtr::Managed(managed) => managed,
                    _ => return Err(StackError::ExpectedManaged),
                };

                if slot & 0b11 == Slot::$constant {
                    Ok((slot >> 2) as usize)
                } else {
                    Err(StackError::IncompatibleSlot)
                }
            }
        }
    };
}

decl_managed!(StringSlot, STRING);
decl_managed!(ArraySlot, ARRAY);
decl_managed!(ExternalSlot, EXTERNAL);

/// Trait for converting into managed slots.
pub(super) trait IntoSlot {
    /// Convert thing into a managed slot.
    fn into_slot(value: ValuePtr) -> Result<usize, StackError>;
}

#[cfg(test)]
mod tests {
    use super::Slot;

    #[test]
    fn test_slot() {
        assert_eq!(Slot::string(4).into_managed(), (crate::Managed::String, 4));
        assert_eq!(Slot::array(4).into_managed(), (crate::Managed::Array, 4));
        assert_eq!(
            Slot::external(4).into_managed(),
            (crate::Managed::External, 4)
        );
    }
}
