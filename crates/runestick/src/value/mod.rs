mod value_type;
mod value_type_info;

pub use self::value_type::ValueType;
pub use self::value_type_info::ValueTypeInfo;
use crate::access;
use crate::any::Any;
use crate::bytes::Bytes;
use crate::future::Future;
use crate::hash::Hash;
use crate::shared;
use crate::shared::Shared;
use crate::shared_ptr::SharedPtr;
use crate::vm::VmError;
use std::any;
use std::rc::Rc;

/// The type of an object.
pub type Object<T> = crate::collections::HashMap<String, T>;

/// A helper type to deserialize arrays with different interior types.
///
/// This implements [FromValue], allowing it to be used as a return value from
/// a virtual machine.
///
/// [FromValue]: crate::FromValue
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VecTuple<I>(pub I);

/// A tuple with a well-defined type.
#[derive(Debug)]
pub struct TypedTuple {
    /// The type hash of the tuple.
    pub ty: Hash,
    /// Content of the tuple.
    pub tuple: Box<[Value]>,
}

/// An object with a well-defined type.
#[derive(Debug)]
pub struct TypedObject {
    /// The type hash of the object.
    pub ty: Hash,
    /// Content of the object.
    pub object: Object<Value>,
}

/// An entry on the stack.
#[derive(Debug, Clone)]
pub enum Value {
    /// The unit value.
    Unit,
    /// A boolean.
    Bool(bool),
    /// A single byte.
    Byte(u8),
    /// A character.
    Char(char),
    /// A number.
    Integer(i64),
    /// A float.
    Float(f64),
    /// A type.
    Type(Hash),
    /// A static string.
    ///
    /// While `Rc<str>` would've been enough to store an unsized `str`, either
    /// `Box<str>` or `String` must be used to reduce the size of the type to
    /// 8 bytes, to ensure that a stack value is 16 bytes in size.
    ///
    /// `Rc<str>` on the other hand wraps a so-called fat pointer, which is 16
    /// bytes.
    StaticString(Rc<String>),
    /// A UTF-8 string.
    String(Shared<String>),
    /// A byte string.
    Bytes(Shared<Bytes>),
    /// A vector containing any values.
    Vec(Shared<Vec<Value>>),
    /// A tuple.
    Tuple(Shared<Box<[Value]>>),
    /// An object.
    Object(Shared<Object<Value>>),
    /// A stored future.
    Future(Shared<Future>),
    /// An empty value indicating nothing.
    Option(Shared<Option<Value>>),
    /// A stored result in a slot.
    Result(Shared<Result<Value, Value>>),
    /// A tuple with a well-defined type.
    TypedTuple(Shared<TypedTuple>),
    /// An object with a well-defined type.
    TypedObject(Shared<TypedObject>),
    /// An external value.
    External(Shared<Any>),
    /// A shared pointer. This is what's used when a reference is passed into
    /// the Vm, and must absolutely not outlive foreign function calls.
    Ptr(Shared<SharedPtr>),
}

impl Value {
    /// Cosntruct a value from a raw pointer.
    ///
    /// # Safety
    ///
    /// The returned value mustn't be used after it's been freed.
    pub unsafe fn from_ptr<T>(ptr: &T) -> Self
    where
        T: any::Any,
    {
        Self::Ptr(Shared::new(SharedPtr::from_ptr(ptr)))
    }

    /// Cosntruct a value from a raw mutable pointer.
    ///
    /// # Safety
    ///
    /// The returned value mustn't be used after it's been freed.
    pub unsafe fn from_mut_ptr<T>(ptr: &mut T) -> Self
    where
        T: any::Any,
    {
        Self::Ptr(Shared::new(SharedPtr::from_mut_ptr(ptr)))
    }

    /// Try to coerce value reference into a boolean.
    #[inline]
    pub fn into_bool(self) -> Result<bool, VmError> {
        match self {
            Self::Bool(b) => Ok(b),
            actual => Err(VmError::ExpectedBoolean {
                actual: actual.type_info()?,
            }),
        }
    }

    /// Try to coerce value reference into a result.
    #[inline]
    pub fn into_result(self) -> Result<Shared<Result<Value, Value>>, VmError> {
        match self {
            Self::Result(result) => Ok(result),
            actual => Err(VmError::ExpectedResult {
                actual: actual.type_info()?,
            }),
        }
    }

    /// Try to coerce value reference into an option.
    #[inline]
    pub fn into_option(self) -> Result<Shared<Option<Value>>, VmError> {
        match self {
            Self::Option(option) => Ok(option),
            actual => Err(VmError::ExpectedOption {
                actual: actual.type_info()?,
            }),
        }
    }

    /// Try to coerce value reference into a string.
    #[inline]
    pub fn into_string(self) -> Result<Shared<String>, VmError> {
        match self {
            Self::String(string) => Ok(string),
            actual => Err(VmError::ExpectedString {
                actual: actual.type_info()?,
            }),
        }
    }

    /// Try to coerce value reference into bytes.
    #[inline]
    pub fn into_bytes(self) -> Result<Shared<Bytes>, VmError> {
        match self {
            Self::Bytes(bytes) => Ok(bytes),
            actual => Err(VmError::ExpectedBytes {
                actual: actual.type_info()?,
            }),
        }
    }

    /// Try to coerce value reference into a vector.
    #[inline]
    pub fn into_vec(self) -> Result<Shared<Vec<Value>>, VmError> {
        match self {
            Self::Vec(vec) => Ok(vec),
            actual => Err(VmError::ExpectedVec {
                actual: actual.type_info()?,
            }),
        }
    }

    /// Try to coerce value reference into a tuple.
    #[inline]
    pub fn into_tuple(self) -> Result<Shared<Box<[Value]>>, VmError> {
        match self {
            Self::Tuple(tuple) => Ok(tuple),
            actual => Err(VmError::ExpectedTuple {
                actual: actual.type_info()?,
            }),
        }
    }

    /// Try to coerce value reference into an object.
    #[inline]
    pub fn into_object(self) -> Result<Shared<Object<Value>>, VmError> {
        match self {
            Self::Object(object) => Ok(object),
            actual => Err(VmError::ExpectedObject {
                actual: actual.type_info()?,
            }),
        }
    }

    /// Try to coerce value reference into an external.
    #[inline]
    pub fn into_external(self) -> Result<Shared<Any>, VmError> {
        match self {
            Self::External(any) => Ok(any),
            actual => Err(VmError::ExpectedExternal {
                actual: actual.type_info()?,
            }),
        }
    }

    /// Try to coerce value into an external ref and an associated guard.
    #[inline]
    pub unsafe fn unsafe_into_external_ref<T>(self) -> Result<(*const T, RawValueRefGuard), VmError>
    where
        T: any::Any,
    {
        match self {
            Self::External(external) => {
                let external = external.downcast_strong_ref::<T>()?;
                let (data, guard) = shared::StrongRef::into_raw(external);
                let guard = RawValueRefGuard::RawStrongRefGuard(guard);
                Ok((data, guard))
            }
            Self::Ptr(ptr) => {
                let ptr = ptr.downcast_ref::<T>()?;
                let (data, guard) = access::Ref::into_raw(ptr);
                let guard = RawValueRefGuard::RawRefGuard(guard);
                Ok((data, guard))
            }
            actual => Err(VmError::ExpectedExternal {
                actual: actual.type_info()?,
            }),
        }
    }

    /// Try to coerce value into an external ref and an associated guard.
    #[inline]
    pub unsafe fn unsafe_into_external_mut<T>(self) -> Result<(*mut T, RawValueMutGuard), VmError>
    where
        T: any::Any,
    {
        match self {
            Self::External(external) => {
                let external = external.downcast_strong_mut::<T>()?;
                let (data, guard) = shared::StrongMut::into_raw(external);
                let guard = RawValueMutGuard::RawStrongMutGuard(guard);
                Ok((data, guard))
            }
            Self::Ptr(ptr) => {
                let ptr = ptr.downcast_mut::<T>()?;
                let (data, guard) = access::Mut::into_raw(ptr);
                let guard = RawValueMutGuard::RawMutGuard(guard);
                Ok((data, guard))
            }
            actual => Err(VmError::ExpectedExternal {
                actual: actual.type_info()?,
            }),
        }
    }

    /// Get the type information for the current value.
    pub fn value_type(&self) -> Result<ValueType, VmError> {
        Ok(match self {
            Self::Unit => ValueType::Unit,
            Self::Bool(..) => ValueType::Bool,
            Self::Byte(..) => ValueType::Byte,
            Self::Char(..) => ValueType::Char,
            Self::Integer(..) => ValueType::Integer,
            Self::Float(..) => ValueType::Float,
            Self::StaticString(..) => ValueType::String,
            Self::String(..) => ValueType::String,
            Self::Bytes(..) => ValueType::Bytes,
            Self::Vec(..) => ValueType::Vec,
            Self::Tuple(..) => ValueType::Tuple,
            Self::Object(..) => ValueType::Object,
            Self::Type(..) => ValueType::Type,
            Self::Future(..) => ValueType::Future,
            Self::Result(..) => ValueType::Result,
            Self::Option(..) => ValueType::Option,
            Self::TypedObject(object) => ValueType::TypedObject(object.get_ref()?.ty),
            Self::TypedTuple(tuple) => ValueType::TypedTuple(tuple.get_ref()?.ty),
            Self::External(any) => ValueType::External(any.get_ref()?.type_id()),
            Self::Ptr(ptr) => ValueType::External(ptr.get_ref()?.type_id()),
        })
    }

    /// Get the type information for the current value.
    pub fn type_info(&self) -> Result<ValueTypeInfo, VmError> {
        Ok(match self {
            Self::Unit => ValueTypeInfo::Unit,
            Self::Bool(..) => ValueTypeInfo::Bool,
            Self::Byte(..) => ValueTypeInfo::Byte,
            Self::Char(..) => ValueTypeInfo::Char,
            Self::Integer(..) => ValueTypeInfo::Integer,
            Self::Float(..) => ValueTypeInfo::Float,
            Self::StaticString(..) => ValueTypeInfo::String,
            Self::String(..) => ValueTypeInfo::String,
            Self::Bytes(..) => ValueTypeInfo::Bytes,
            Self::Vec(..) => ValueTypeInfo::Vec,
            Self::Tuple(..) => ValueTypeInfo::Tuple,
            Self::Object(..) => ValueTypeInfo::Object,
            Self::Type(hash) => ValueTypeInfo::Type(*hash),
            Self::Future(..) => ValueTypeInfo::Future,
            Self::Option(..) => ValueTypeInfo::Option,
            Self::Result(..) => ValueTypeInfo::Result,
            Self::TypedObject(object) => ValueTypeInfo::TypedObject(object.get_ref()?.ty),
            Self::TypedTuple(tuple) => ValueTypeInfo::TypedTuple(tuple.get_ref()?.ty),
            Self::External(external) => ValueTypeInfo::External(external.get_ref()?.type_name()),
            Self::Ptr(ptr) => ValueTypeInfo::External(ptr.get_ref()?.type_name()),
        })
    }
}

/// A raw guard for a reference to a value.
pub enum RawValueRefGuard {
    /// The guard from an internally held value.
    RawStrongRefGuard(shared::RawStrongRefGuard),
    /// The guard from an external reference.
    RawRefGuard(access::RawRefGuard),
}

/// A raw guard for a reference to a value.
pub enum RawValueMutGuard {
    /// The guard from an internally held value.
    RawStrongMutGuard(shared::RawStrongMutGuard),
    /// The guard from an external reference.
    RawMutGuard(access::RawMutGuard),
}

#[cfg(test)]
mod tests {
    use super::Value;

    #[test]
    fn test_size() {
        // :( - make this 16 bytes again by reducing the size of the Rc.
        assert_eq! {
            std::mem::size_of::<Value>(),
            16,
        };
    }
}
