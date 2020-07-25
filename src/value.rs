use crate::vm::{FnHash, Vm};
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

/// An entry on the stack.
#[derive(Debug, Clone, Copy)]
pub enum Value {
    /// An empty unit.
    Unit,
    /// A string.
    String(usize),
    /// An array.
    Array(usize),
    /// A number.
    Integer(i64),
    /// A float.
    Float(f64),
    /// A boolean.
    Bool(bool),
    /// Reference to a foreign type.
    External(usize),
    /// Reference to an internal function.
    Fn(FnHash),
}

impl Value {
    /// Get the type information for the current value.
    pub fn value_type(&self, vm: &Vm) -> Result<ValueType, ExternalTypeError> {
        Ok(match *self {
            Self::Unit => ValueType::Unit,
            Self::String(..) => ValueType::String,
            Self::Array(..) => ValueType::Array,
            Self::Integer(..) => ValueType::Integer,
            Self::Float(..) => ValueType::Float,
            Self::Bool(..) => ValueType::Bool,
            Self::External(external) => {
                let (_, type_hash) = vm
                    .external_type(external)
                    .ok_or_else(|| ExternalTypeError(external))?;

                ValueType::External(type_hash)
            }
            Self::Fn(fn_hash) => ValueType::Fn(fn_hash),
        })
    }

    /// Get the type information for the current value.
    pub fn type_info(&self, vm: &Vm) -> Result<ValueTypeInfo, ExternalTypeError> {
        Ok(match *self {
            Self::Unit => ValueTypeInfo::Unit,
            Self::String(..) => ValueTypeInfo::String,
            Self::Array(..) => ValueTypeInfo::Array,
            Self::Integer(..) => ValueTypeInfo::Integer,
            Self::Float(..) => ValueTypeInfo::Float,
            Self::Bool(..) => ValueTypeInfo::Bool,
            Self::External(external) => {
                let (type_name, type_hash) = vm
                    .external_type(external)
                    .ok_or_else(|| ExternalTypeError(external))?;

                ValueTypeInfo::External(type_name, type_hash)
            }
            Self::Fn(fn_hash) => ValueTypeInfo::Fn(fn_hash),
        })
    }
}

impl Default for Value {
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
    /// Reference to an internal function.
    Fn(FnHash),
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
    /// Reference to an internal function.
    Fn(FnHash),
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
            Self::Fn(hash) => write!(fmt, "Fn({})", hash),
        }
    }
}
