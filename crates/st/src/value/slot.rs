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

    /// Get the slot as an usize.
    pub fn into_usize(self) -> usize {
        (self.0 >> 2) as usize
    }

    /// Convert into its managed variant.
    pub fn into_managed(self) -> Managed {
        match self.0 & 0b11 {
            Self::STRING => Managed::String,
            Self::ARRAY => Managed::Array,
            Self::OBJECT => Managed::Object,
            Self::EXTERNAL => Managed::External,
            other => panic!("impossible slot: {}", other),
        }
    }

    /// Construct a string slot.
    pub fn string(slot: usize) -> Self {
        Self((slot << 2) as u64 | Self::STRING)
    }

    /// Construct an array slot.
    pub fn array(slot: usize) -> Self {
        Self((slot << 2) as u64 | Self::ARRAY)
    }

    /// Construct an object slot.
    pub fn object(slot: usize) -> Self {
        Self((slot << 2) as u64 | Self::OBJECT)
    }

    /// Construct an external slot.
    pub fn external(slot: usize) -> Self {
        Self((slot << 2) as u64 | Self::EXTERNAL)
    }
}

impl fmt::Debug for Slot {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        let managed = self.into_managed();
        write!(fmt, "{:?}({:?})", managed, self.into_usize())
    }
}

impl fmt::Display for Slot {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        let managed = self.into_managed();
        write!(fmt, "{}({})", managed, self.into_usize())
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

                if slot.0 & 0b11 == Slot::$constant {
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
        assert_eq!(Slot::string(77).into_managed(), Managed::String);
        assert_eq!(Slot::string(77).into_usize(), 77);
        assert_eq!(Slot::array(78).into_managed(), Managed::Array);
        assert_eq!(Slot::array(78).into_usize(), 78);
        assert_eq!(Slot::object(79).into_managed(), Managed::Object);
        assert_eq!(Slot::object(79).into_usize(), 79);
        assert_eq!(Slot::external(80).into_managed(), Managed::External);
        assert_eq!(Slot::external(80).into_usize(), 80);
    }
}
