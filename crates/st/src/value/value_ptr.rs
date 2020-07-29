use crate::value::slot;
use crate::value::slot::{IntoSlot, Slot};
use crate::value::{Managed, ValueType, ValueTypeInfo};
use crate::vm::{StackError, Vm};

/// An entry on the stack.
#[derive(Debug, Clone, Copy)]
pub enum ValuePtr {
    /// An empty unit.
    Unit,
    /// A number.
    Integer(i64),
    /// A float.
    Float(f64),
    /// A boolean.
    Bool(bool),
    /// A character.
    Char(char),
    /// A managed reference.
    Managed(Slot),
}

impl ValuePtr {
    /// Convert value into a managed.
    #[inline]
    pub fn into_managed(self) -> Result<(Managed, usize), StackError> {
        if let Self::Managed(slot) = self {
            Ok(slot.into_managed())
        } else {
            Err(StackError::ExpectedManaged)
        }
    }

    /// Try to convert into managed.
    #[inline]
    pub fn try_into_managed(self) -> Option<(Managed, usize)> {
        if let Self::Managed(slot) = self {
            Some(slot.into_managed())
        } else {
            None
        }
    }

    /// Convert value into a managed slot.
    #[inline]
    fn into_slot<T>(self) -> Result<usize, StackError>
    where
        T: IntoSlot,
    {
        T::into_slot(self)
    }

    /// Try to coerce value reference into an external.
    pub fn into_external(self) -> Result<usize, StackError> {
        self.into_slot::<slot::ExternalSlot>()
    }

    /// Try to coerce value reference into an array.
    pub fn into_array(self) -> Result<usize, StackError> {
        self.into_slot::<slot::ArraySlot>()
    }

    /// Try to coerce value reference into an array.
    pub fn into_string(self) -> Result<usize, StackError> {
        self.into_slot::<slot::StringSlot>()
    }

    /// Get the type information for the current value.
    pub fn value_type(&self, vm: &Vm) -> Result<ValueType, StackError> {
        Ok(match *self {
            Self::Unit => ValueType::Unit,
            Self::Integer(..) => ValueType::Integer,
            Self::Float(..) => ValueType::Float,
            Self::Bool(..) => ValueType::Bool,
            Self::Char(..) => ValueType::Char,
            Self::Managed(slot) => match slot.into_managed() {
                (Managed::String, ..) => ValueType::String,
                (Managed::Array, _) => ValueType::Array,
                (Managed::Object, _) => ValueType::Object,
                (Managed::External, slot) => {
                    let (_, type_hash) = vm.external_type(slot)?;

                    ValueType::External(type_hash)
                }
            },
        })
    }

    /// Get the type information for the current value.
    pub fn type_info(&self, vm: &Vm) -> Result<ValueTypeInfo, StackError> {
        Ok(match *self {
            Self::Unit => ValueTypeInfo::Unit,
            Self::Integer(..) => ValueTypeInfo::Integer,
            Self::Float(..) => ValueTypeInfo::Float,
            Self::Bool(..) => ValueTypeInfo::Bool,
            Self::Char(..) => ValueTypeInfo::Char,
            Self::Managed(slot) => match slot.into_managed() {
                (Managed::String, _) => ValueTypeInfo::String,
                (Managed::Array, _) => ValueTypeInfo::Array,
                (Managed::Object, _) => ValueTypeInfo::Object,
                (Managed::External, slot) => {
                    let (type_name, _) = vm.external_type(slot)?;
                    ValueTypeInfo::External(type_name)
                }
            },
        })
    }
}

impl Default for ValuePtr {
    fn default() -> Self {
        Self::Unit
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
