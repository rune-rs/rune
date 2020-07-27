use crate::external::External;
use crate::vm::Vm;
use std::any::{Any, TypeId};
use std::fmt;
use thiserror::Error;

/// Error raised when external type cannot be resolved.
#[derive(Debug, Error)]
#[error("failed to resolve external at slot `{0}`")]
pub struct ExternalTypeError(usize);

/// The hash of a foreign type.
#[derive(Clone, Debug, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TypeHash(TypeId);

impl TypeHash {
    /// Construct a new TypeHash.
    pub(crate) fn new(type_id: TypeId) -> Self {
        Self(type_id)
    }

    /// Construct a hash for the given type.
    pub fn of<T>() -> Self
    where
        T: Any,
    {
        Self(TypeId::of::<T>())
    }
}

/// Describes what slot error happened.
#[derive(Debug, Clone, Copy)]
pub enum ValueError {
    /// A string slot could not be looked up.
    String(usize),
    /// An array slot could not be looked up.
    Array(usize),
    /// An external could not be looked up.
    External(usize),
    /// A dynamic value could not be looked up.
    Value(usize),
}

#[derive(Debug)]
/// An owned value.
pub enum Value {
    /// An empty unit.
    Unit,
    /// A string.
    String(Box<str>),
    /// An array.
    Array(Box<[Value]>),
    /// An integer.
    Integer(i64),
    /// A float.
    Float(f64),
    /// A boolean.
    Bool(bool),
    /// Reference to an external type.
    External(Box<dyn External>),
    /// A slot error value where we were unable to convert a value reference
    /// from a slot.
    Error(ValueError),
}

impl Clone for Value {
    fn clone(&self) -> Self {
        match self {
            Self::Unit => Self::Unit,
            Self::String(string) => Self::String(string.clone()),
            Self::Array(array) => Self::Array(array.clone()),
            Self::Integer(integer) => Self::Integer(*integer),
            Self::Float(float) => Self::Float(*float),
            Self::Bool(boolean) => Self::Bool(*boolean),
            Self::External(external) => Self::External(external.as_ref().clone_external()),
            Self::Error(error) => Self::Error(*error),
        }
    }
}

/// Managed entries on the stack.
#[derive(Debug, Clone, Copy)]
pub enum Managed {
    /// A string.
    String(usize),
    /// An array.
    Array(usize),
    /// Reference to an external type.
    External(usize),
}

/// An entry on the stack.
#[derive(Debug, Clone, Copy)]
pub enum ValueRef {
    /// An empty unit.
    Unit,
    /// A number.
    Integer(i64),
    /// A float.
    Float(f64),
    /// A boolean.
    Bool(bool),
    /// A managed reference.
    Managed(Managed),
}

impl ValueRef {
    /// Get the type information for the current value.
    pub fn value_type(&self, vm: &Vm) -> Result<ValueType, ExternalTypeError> {
        Ok(match *self {
            Self::Unit => ValueType::Unit,
            Self::Integer(..) => ValueType::Integer,
            Self::Float(..) => ValueType::Float,
            Self::Bool(..) => ValueType::Bool,
            Self::Managed(managed) => match managed {
                Managed::String(..) => ValueType::String,
                Managed::Array(..) => ValueType::Array,
                Managed::External(external) => {
                    let (_, type_hash) = vm
                        .external_type(external)
                        .ok_or_else(|| ExternalTypeError(external))?;

                    ValueType::External(type_hash)
                }
            },
        })
    }

    /// Get the type information for the current value.
    pub fn type_info(&self, vm: &Vm) -> Result<ValueTypeInfo, ExternalTypeError> {
        Ok(match *self {
            Self::Unit => ValueTypeInfo::Unit,
            Self::Integer(..) => ValueTypeInfo::Integer,
            Self::Float(..) => ValueTypeInfo::Float,
            Self::Bool(..) => ValueTypeInfo::Bool,
            Self::Managed(managed) => match managed {
                Managed::String(..) => ValueTypeInfo::String,
                Managed::Array(..) => ValueTypeInfo::Array,
                Managed::External(slot) => {
                    let (type_name, type_hash) = vm
                        .external_type(slot)
                        .ok_or_else(|| ExternalTypeError(slot))?;

                    ValueTypeInfo::External(type_name, type_hash)
                }
            },
        })
    }
}

impl Default for ValueRef {
    fn default() -> Self {
        Self::Unit
    }
}

/// The type of an entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ValueType {
    /// An empty unit.
    Unit,
    /// A string.
    String,
    /// An array of dynamic values.
    Array,
    /// A number.
    Integer,
    /// A float.
    Float,
    /// A boolean.
    Bool,
    /// Reference to a foreign type.
    External(TypeHash),
}

/// Type information about a value, that can be printed for human consumption
/// through its [Display][fmt::Display] implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ValueTypeInfo {
    /// An empty unit.
    Unit,
    /// A string.
    String,
    /// An array.
    Array,
    /// A number.
    Integer,
    /// A float.
    Float,
    /// A boolean.
    Bool,
    /// Reference to a foreign type.
    External(&'static str, TypeHash),
}

impl fmt::Display for ValueTypeInfo {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Unit => write!(fmt, "()"),
            Self::String => write!(fmt, "String"),
            Self::Array => write!(fmt, "Array"),
            Self::Integer => write!(fmt, "Integer"),
            Self::Float => write!(fmt, "Float"),
            Self::Bool => write!(fmt, "Bool"),
            Self::External(name, _) => write!(fmt, "External({})", name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ValueRef;

    #[test]
    fn test_size() {
        assert_eq! {
            std::mem::size_of::<ValueRef>(),
            16
        };
    }
}
