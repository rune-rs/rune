use crate::value::{Managed, ValuePtr};
use crate::vm::{StackError, Vm};
use std::fmt;

/// Compact information on typed slot.
#[derive(Clone, Copy, PartialEq)]
pub struct Slot(u64);

impl Slot {
    const STRING: u64 = 0;
    const ARRAY: u64 = 1;
    const OBJECT: u64 = 2;
    const EXTERNAL: u64 = 3;
    const TYPE_MASK: u64 = 0b11;
    // 30 bits of position information.
    const POS_MASK: u64 = 0xff_ff_ff_fc;
    // 32 bits of generation information.
    const GEN_MASK: u64 = 0xff_ff_ff_ff << 32;

    /// Get the slot as an usize.
    #[inline]
    pub fn into_usize(self) -> usize {
        ((self.0 & Self::POS_MASK) >> 2) as usize
    }

    /// Get the generation of the slot.
    #[inline]
    pub fn into_generation(self) -> usize {
        ((self.0 & Self::GEN_MASK) >> 32) as usize
    }

    /// Convert into its managed variant.
    #[inline]
    pub fn into_managed(self) -> Managed {
        match self.0 & Self::TYPE_MASK {
            Self::STRING => Managed::String,
            Self::ARRAY => Managed::Array,
            Self::OBJECT => Managed::Object,
            Self::EXTERNAL => Managed::External,
            other => panic!("impossible slot: {}", other),
        }
    }

    /// Construct a string slot.
    #[inline]
    pub fn string(gen: usize, slot: usize) -> Self {
        Self(((gen as u64) << 32) | (slot << 2) as u64 | Self::STRING)
    }

    /// Construct an array slot.
    #[inline]
    pub fn array(gen: usize, slot: usize) -> Self {
        Self(((gen as u64) << 32) | (slot << 2) as u64 | Self::ARRAY)
    }

    /// Construct an object slot.
    #[inline]
    pub fn object(gen: usize, slot: usize) -> Self {
        Self(((gen as u64) << 32) | (slot << 2) as u64 | Self::OBJECT)
    }

    /// Construct an external slot.
    #[inline]
    pub fn external(gen: usize, slot: usize) -> Self {
        Self(((gen as u64) << 32) | (slot << 2) as u64 | Self::EXTERNAL)
    }
}

impl fmt::Debug for Slot {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        let managed = self.into_managed();
        write!(
            fmt,
            "{:?}({:?}, {:?})",
            managed,
            self.into_generation(),
            self.into_usize()
        )
    }
}

impl fmt::Display for Slot {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        let managed = self.into_managed();
        write!(
            fmt,
            "{}({}, {})",
            managed,
            self.into_generation(),
            self.into_usize()
        )
    }
}

macro_rules! decl_managed {
    ($name:ident, $constant:ident, $expected:ident) => {
        #[allow(unused)]
        pub(super) struct $name(());

        impl IntoSlot for $name {
            fn into_slot(value: ValuePtr, vm: &Vm) -> Result<Slot, StackError> {
                let slot = match value {
                    ValuePtr::Managed(slot) => slot,
                    actual => {
                        let actual = actual.type_info(vm)?;
                        return Err(StackError::$expected { actual });
                    }
                };

                if slot.0 & Slot::TYPE_MASK == Slot::$constant {
                    Ok(slot)
                } else {
                    Err(StackError::IncompatibleSlot)
                }
            }
        }
    };
}

decl_managed!(StringSlot, STRING, ExpectedString);
decl_managed!(ArraySlot, ARRAY, ExpectedArray);
decl_managed!(ObjectSlot, OBJECT, ExpectedObject);
decl_managed!(ExternalSlot, EXTERNAL, ExpectedExternal);

/// Trait for converting into managed slots.
pub(super) trait IntoSlot {
    /// Convert thing into a managed slot.
    fn into_slot(value: ValuePtr, vm: &Vm) -> Result<Slot, StackError>;
}

#[cfg(test)]
mod tests {
    use super::Slot;
    use crate::value::Managed;

    #[test]
    fn test_slot() {
        assert_eq!(Slot::string(5, 77).into_managed(), Managed::String);
        assert_eq!(Slot::string(5, 77).into_usize(), 77);
        assert_eq!(Slot::string(5, 77).into_generation(), 5);
        assert_eq!(Slot::array(5, 78).into_managed(), Managed::Array);
        assert_eq!(Slot::array(5, 78).into_usize(), 78);
        assert_eq!(Slot::array(6, 78).into_generation(), 6);
        assert_eq!(Slot::object(7, 79).into_managed(), Managed::Object);
        assert_eq!(Slot::object(7, 79).into_usize(), 79);
        assert_eq!(Slot::object(7, 79).into_generation(), 7);
        assert_eq!(Slot::external(8, 80).into_managed(), Managed::External);
        assert_eq!(Slot::external(8, 80).into_usize(), 80);
        assert_eq!(Slot::external(8, 80).into_generation(), 8);
    }
}
