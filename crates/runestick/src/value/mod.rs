mod owned_value;
mod slot;
mod value_ref;
mod value_type;
mod value_type_info;

pub use self::owned_value::OwnedValue;
pub use self::slot::Slot;
pub use self::value_ref::ValueRef;
pub use self::value_type::ValueType;
pub use self::value_type_info::ValueTypeInfo;

use crate::hash::Hash;
use crate::vm::{Vm, VmError};

/// The type of an object.
pub type Object<T> = crate::collections::HashMap<String, T>;

/// A helper type to deserialize arrays with different interior types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VecTuple<I>(pub I);

/// An entry on the stack.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Value {
    /// The unit value.
    Unit,
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
    /// A vector.
    Vec(Slot),
    /// A tuple.
    Tuple(Slot),
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
    /// An empty value indicating nothing.
    Option(Slot),
    /// A stored result in a slot.
    Result(Slot),
}

impl Value {
    /// Try to coerce value reference into a result.
    #[inline]
    pub fn into_result(self, vm: &Vm) -> Result<Slot, VmError> {
        match self {
            Self::Result(slot) => Ok(slot),
            actual => Err(VmError::ExpectedResult {
                actual: actual.type_info(vm)?,
            }),
        }
    }

    /// Try to coerce value reference into an option.
    #[inline]
    pub fn into_option(self, vm: &Vm) -> Result<Slot, VmError> {
        match self {
            Self::Option(slot) => Ok(slot),
            actual => Err(VmError::ExpectedOption {
                actual: actual.type_info(vm)?,
            }),
        }
    }

    /// Try to coerce value reference into a string.
    #[inline]
    pub fn into_string(self, vm: &Vm) -> Result<Slot, VmError> {
        match self {
            Self::String(slot) => Ok(slot),
            actual => Err(VmError::ExpectedString {
                actual: actual.type_info(vm)?,
            }),
        }
    }

    /// Try to coerce value reference into a vector.
    #[inline]
    pub fn into_vec(self, vm: &Vm) -> Result<Slot, VmError> {
        match self {
            Self::Vec(slot) => Ok(slot),
            actual => Err(VmError::ExpectedVec {
                actual: actual.type_info(vm)?,
            }),
        }
    }

    /// Try to coerce value reference into a tuple.
    #[inline]
    pub fn into_tuple(self, vm: &Vm) -> Result<Slot, VmError> {
        match self {
            Self::Tuple(slot) => Ok(slot),
            actual => Err(VmError::ExpectedTuple {
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
            Self::Unit => ValueType::Unit,
            Self::Integer(..) => ValueType::Integer,
            Self::Float(..) => ValueType::Float,
            Self::Bool(..) => ValueType::Bool,
            Self::Char(..) => ValueType::Char,
            Self::String(..) => ValueType::String,
            Self::StaticString(..) => ValueType::String,
            Self::Vec(..) => ValueType::Vec,
            Self::Tuple(..) => ValueType::Tuple,
            Self::Object(..) => ValueType::Object,
            Self::External(slot) => ValueType::External(vm.slot_type_id(slot)?),
            Self::Type(..) => ValueType::Type,
            Self::Fn(hash) => ValueType::Fn(hash),
            Self::Future(..) => ValueType::Future,
            Self::Result(..) => ValueType::Result,
            Self::Option(..) => ValueType::Option,
        })
    }

    /// Get the type information for the current value.
    pub fn type_info(&self, vm: &Vm) -> Result<ValueTypeInfo, VmError> {
        Ok(match *self {
            Self::Unit => ValueTypeInfo::Unit,
            Self::Integer(..) => ValueTypeInfo::Integer,
            Self::Float(..) => ValueTypeInfo::Float,
            Self::Bool(..) => ValueTypeInfo::Bool,
            Self::Char(..) => ValueTypeInfo::Char,
            Self::String(..) => ValueTypeInfo::String,
            Self::StaticString(..) => ValueTypeInfo::String,
            Self::Vec(..) => ValueTypeInfo::Vec,
            Self::Tuple(..) => ValueTypeInfo::Tuple,
            Self::Object(..) => ValueTypeInfo::Object,
            Self::External(slot) => ValueTypeInfo::External(vm.slot_type_name(slot)?),
            Self::Type(..) => ValueTypeInfo::Type,
            Self::Fn(hash) => ValueTypeInfo::Fn(hash),
            Self::Future(..) => ValueTypeInfo::Future,
            Self::Option(..) => ValueTypeInfo::Option,
            Self::Result(..) => ValueTypeInfo::Result,
        })
    }
}

impl Default for Value {
    fn default() -> Self {
        Self::Unit
    }
}

#[cfg(test)]
mod tests {
    use super::Value;

    #[test]
    fn test_size() {
        assert_eq! {
            std::mem::size_of::<Value>(),
            16,
        };
    }
}
