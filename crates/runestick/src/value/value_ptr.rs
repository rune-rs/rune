use crate::hash::Hash;
use crate::value::slot::Slot;
use crate::value::{ValueType, ValueTypeInfo};
use crate::vm::{Vm, VmError};

/// An entry on the stack.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ValuePtr {
    /// An empty value indicating nothing.
    None,
    /// A boolean.
    Bool(bool),
    /// A character.
    Char(char),
    /// A number.
    Integer(i64),
    /// A float.
    Float(f64),
    /// A static string.
    /// The index is the index into the static string slot for the current unit.
    StaticString(usize),
    /// A String.
    String(Slot),
    /// An array.
    Array(Slot),
    /// An object.
    Object(Slot),
    /// An external value.
    External(Slot),
    /// A type.
    Type(Hash),
    /// A function pointer.
    Fn(Hash),
    /// A stored future.
    Future(Slot),
}

impl ValuePtr {
    /// Try to coerce value reference into an array.
    #[inline]
    pub fn into_string(self, vm: &Vm) -> Result<Slot, VmError> {
        match self {
            Self::String(slot) => Ok(slot),
            actual => Err(VmError::ExpectedString {
                actual: actual.type_info(vm)?,
            }),
        }
    }

    /// Try to coerce value reference into an array.
    #[inline]
    pub fn into_array(self, vm: &Vm) -> Result<Slot, VmError> {
        match self {
            Self::Array(slot) => Ok(slot),
            actual => Err(VmError::ExpectedArray {
                actual: actual.type_info(vm)?,
            }),
        }
    }

    /// Try to coerce value reference into an object.
    #[inline]
    pub fn into_object(self, vm: &Vm) -> Result<Slot, VmError> {
        match self {
            Self::Object(slot) => Ok(slot),
            actual => Err(VmError::ExpectedObject {
                actual: actual.type_info(vm)?,
            }),
        }
    }

    /// Try to coerce value reference into an external.
    #[inline]
    pub fn into_external(self, vm: &Vm) -> Result<Slot, VmError> {
        match self {
            Self::External(slot) => Ok(slot),
            actual => Err(VmError::ExpectedExternal {
                actual: actual.type_info(vm)?,
            }),
        }
    }

    /// Get the type information for the current value.
    pub fn value_type(&self, vm: &Vm) -> Result<ValueType, VmError> {
        Ok(match *self {
            Self::None => ValueType::Unit,
            Self::Integer(..) => ValueType::Integer,
            Self::Float(..) => ValueType::Float,
            Self::Bool(..) => ValueType::Bool,
            Self::Char(..) => ValueType::Char,
            Self::String(..) => ValueType::String,
            Self::StaticString(..) => ValueType::String,
            Self::Array(..) => ValueType::Array,
            Self::Object(..) => ValueType::Object,
            Self::External(slot) => ValueType::External(vm.slot_type_id(slot)?),
            Self::Type(..) => ValueType::Type,
            Self::Fn(hash) => ValueType::Fn(hash),
            Self::Future(..) => ValueType::Future,
        })
    }

    /// Get the type information for the current value.
    pub fn type_info(&self, vm: &Vm) -> Result<ValueTypeInfo, VmError> {
        Ok(match *self {
            Self::None => ValueTypeInfo::Unit,
            Self::Integer(..) => ValueTypeInfo::Integer,
            Self::Float(..) => ValueTypeInfo::Float,
            Self::Bool(..) => ValueTypeInfo::Bool,
            Self::Char(..) => ValueTypeInfo::Char,
            Self::String(..) => ValueTypeInfo::String,
            Self::StaticString(..) => ValueTypeInfo::String,
            Self::Array(..) => ValueTypeInfo::Array,
            Self::Object(..) => ValueTypeInfo::Object,
            Self::External(slot) => ValueTypeInfo::External(vm.slot_type_name(slot)?),
            Self::Type(..) => ValueTypeInfo::Type,
            Self::Fn(hash) => ValueTypeInfo::Fn(hash),
            Self::Future(..) => ValueTypeInfo::Future,
        })
    }
}

impl Default for ValuePtr {
    fn default() -> Self {
        Self::None
    }
}

#[cfg(test)]
mod tests {
    use super::ValuePtr;

    #[test]
    fn test_size() {
        assert_eq! {
            std::mem::size_of::<ValuePtr>(),
            16,
        };
    }
}
