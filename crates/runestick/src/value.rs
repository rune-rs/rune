use crate::{
    AccessError, Any, Bytes, Future, Hash, OwnedMut, OwnedRef, Panic, RawOwnedMut, RawOwnedRef,
    Shared, SharedPtr, ValueType, ValueTypeInfo,
};
use std::any;
use std::fmt;
use std::rc::Rc;
use thiserror::Error;

/// Value raised when interacting with a value.
#[derive(Debug, Error)]
pub enum ValueError {
    /// The virtual machine panicked for a specific reason.
    #[error("panicked `{reason}`")]
    Panic {
        /// The reason for the panic.
        reason: Panic,
    },
    /// Trying to access an inaccessible reference.
    #[error("failed to access value: {error}")]
    AccessError {
        /// Source error.
        #[from]
        error: AccessError,
    },
    /// Error raised when we expected a object.
    #[error("expected a object but found `{actual}`")]
    ExpectedObject {
        /// The actual type observed instead.
        actual: ValueTypeInfo,
    },
    /// Error raised when we expected an external value.
    #[error("expected a external value but found `{actual}`")]
    ExpectedExternal {
        /// The actual type observed instead.
        actual: ValueTypeInfo,
    },
    /// Error raised when we expected a managed value.
    #[error("expected an external, vector, object, or string, but found `{actual}`")]
    ExpectedManaged {
        /// The actual type observed instead.
        actual: ValueTypeInfo,
    },
    /// Error raised when we expected a future.
    #[error("expected future, but found `{actual}`")]
    ExpectedFuture {
        /// The actual type found.
        actual: ValueTypeInfo,
    },
    /// Error raised when expecting a unit.
    #[error("expected unit, but found `{actual}`")]
    ExpectedUnit {
        /// The actual type found.
        actual: ValueTypeInfo,
    },
    /// Error raised when expecting an option.
    #[error("expected option, but found `{actual}`")]
    ExpectedOption {
        /// The actual type found.
        actual: ValueTypeInfo,
    },
    /// Error raised when we expecting a result.
    #[error("expected result, but found `{actual}`")]
    ExpectedResult {
        /// The actual type found.
        actual: ValueTypeInfo,
    },
    /// Error raised when we expected a boolean value.
    #[error("expected booleant, but found `{actual}`")]
    ExpectedBoolean {
        /// The actual type found.
        actual: ValueTypeInfo,
    },
    /// Error raised when we expected a byte value.
    #[error("expected byte, but found `{actual}`")]
    ExpectedByte {
        /// The actual type found.
        actual: ValueTypeInfo,
    },
    /// Error raised when we expected a char value.
    #[error("expected char, but found `{actual}`")]
    ExpectedChar {
        /// The actual type found.
        actual: ValueTypeInfo,
    },
    /// Error raised when an integer value was expected.
    #[error("expected integer, but found `{actual}`")]
    ExpectedInteger {
        /// The actual type found.
        actual: ValueTypeInfo,
    },
    /// Error raised when we expected a float value.
    #[error("expected float, but found `{actual}`")]
    ExpectedFloat {
        /// The actual type found.
        actual: ValueTypeInfo,
    },
    /// Error raised when we expected a string.
    #[error("expected a string but found `{actual}`")]
    ExpectedString {
        /// The actual type observed instead.
        actual: ValueTypeInfo,
    },
    /// Error raised when we expected a byte string.
    #[error("expected a byte string but found `{actual}`")]
    ExpectedBytes {
        /// The actual type observed instead.
        actual: ValueTypeInfo,
    },
    /// Error raised when we expected a vector.
    #[error("expected a vector but found `{actual}`")]
    ExpectedVec {
        /// The actual type observed instead.
        actual: ValueTypeInfo,
    },
    /// Error raised when we expected a tuple.
    #[error("expected a tuple but found `{actual}`")]
    ExpectedTuple {
        /// The actual type observed instead.
        actual: ValueTypeInfo,
    },
    /// Failure to convert a number into an integer.
    #[error("failed to convert value `{from}` to integer `{to}`")]
    ValueToIntegerCoercionError {
        /// Number we tried to convert from.
        from: Integer,
        /// Number type we tried to convert to.
        to: &'static str,
    },
    /// Failure to convert an integer into a value.
    #[error("failed to convert integer `{from}` to value `{to}`")]
    IntegerToValueCoercionError {
        /// Number we tried to convert from.
        from: Integer,
        /// Number type we tried to convert to.
        to: &'static str,
    },
    /// Error raised when we expected an tuple of the given length.
    #[error("expected a tuple of length `{expected}`, but found one with length `{actual}`")]
    ExpectedTupleLength {
        /// The actual length observed.
        actual: usize,
        /// The expected tuple length.
        expected: usize,
    },
    /// Internal error that happens when we run out of items in a list.
    #[error("unexpectedly ran out of items to iterate over")]
    IterationError,
}

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
    pub hash: Hash,
    /// Content of the tuple.
    pub tuple: Box<[Value]>,
}

impl TypedTuple {
    /// Get type info for the typed tuple.
    pub fn type_info(&self) -> ValueTypeInfo {
        ValueTypeInfo::TypedTuple(self.hash)
    }
}

/// A tuple with a well-defined type as a variant of an enum.
#[derive(Debug)]
pub struct VariantTuple {
    /// The type hash of the enum.
    pub enum_hash: Hash,
    /// The variant type hash of the tuple.
    pub hash: Hash,
    /// Content of the tuple.
    pub tuple: Box<[Value]>,
}

impl VariantTuple {
    /// Get type info for the typed tuple.
    pub fn type_info(&self) -> ValueTypeInfo {
        ValueTypeInfo::VariantTuple(self.enum_hash, self.hash)
    }
}

/// An object with a well-defined type.
#[derive(Debug)]
pub struct TypedObject {
    /// The type hash of the object.
    pub hash: Hash,
    /// Content of the object.
    pub object: Object<Value>,
}

impl TypedObject {
    /// Get type info for the typed object.
    pub fn type_info(&self) -> ValueTypeInfo {
        ValueTypeInfo::TypedObject(self.hash)
    }
}

/// An object with a well-defined variant of an enum.
#[derive(Debug)]
pub struct VariantObject {
    /// The type hash of the enum.
    pub enum_hash: Hash,
    /// The type variant hash.
    pub hash: Hash,
    /// Content of the object.
    pub object: Object<Value>,
}

impl VariantObject {
    /// Get type info for the typed object.
    pub fn type_info(&self) -> ValueTypeInfo {
        ValueTypeInfo::VariantObject(self.enum_hash, self.hash)
    }
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
    /// A tuple variant with a well-defined type.
    VariantTuple(Shared<VariantTuple>),
    /// An object with a well-defined type.
    TypedObject(Shared<TypedObject>),
    /// An object variant with a well-defined type.
    VariantObject(Shared<VariantObject>),
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

    /// Try to coerce value into a unit.
    #[inline]
    pub fn into_unit(self) -> Result<(), ValueError> {
        match self {
            Value::Unit => Ok(()),
            actual => Err(ValueError::ExpectedUnit {
                actual: actual.type_info()?,
            }),
        }
    }

    /// Try to coerce value into a boolean.
    #[inline]
    pub fn into_bool(self) -> Result<bool, ValueError> {
        match self {
            Self::Bool(b) => Ok(b),
            actual => Err(ValueError::ExpectedBoolean {
                actual: actual.type_info()?,
            }),
        }
    }

    /// Try to coerce value into a byte.
    #[inline]
    pub fn into_byte(self) -> Result<u8, ValueError> {
        match self {
            Self::Byte(b) => Ok(b),
            actual => Err(ValueError::ExpectedByte {
                actual: actual.type_info()?,
            }),
        }
    }

    /// Try to coerce value into a character.
    #[inline]
    pub fn into_char(self) -> Result<char, ValueError> {
        match self {
            Self::Char(c) => Ok(c),
            actual => Err(ValueError::ExpectedChar {
                actual: actual.type_info()?,
            }),
        }
    }

    /// Try to coerce value into an integer.
    #[inline]
    pub fn into_integer(self) -> Result<i64, ValueError> {
        match self {
            Self::Integer(integer) => Ok(integer),
            actual => Err(ValueError::ExpectedInteger {
                actual: actual.type_info()?,
            }),
        }
    }

    /// Try to coerce value into a float.
    #[inline]
    pub fn into_float(self) -> Result<f64, ValueError> {
        match self {
            Self::Float(float) => Ok(float),
            actual => Err(ValueError::ExpectedFloat {
                actual: actual.type_info()?,
            }),
        }
    }

    /// Try to coerce value into a result.
    #[inline]
    pub fn into_result(self) -> Result<Shared<Result<Value, Value>>, ValueError> {
        match self {
            Self::Result(result) => Ok(result),
            actual => Err(ValueError::ExpectedResult {
                actual: actual.type_info()?,
            }),
        }
    }

    /// Try to coerce value into a future.
    #[inline]
    pub fn into_future(self) -> Result<Shared<Future>, ValueError> {
        match self {
            Value::Future(future) => Ok(future),
            actual => Err(ValueError::ExpectedFuture {
                actual: actual.type_info()?,
            }),
        }
    }

    /// Try to coerce value into an option.
    #[inline]
    pub fn into_option(self) -> Result<Shared<Option<Value>>, ValueError> {
        match self {
            Self::Option(option) => Ok(option),
            actual => Err(ValueError::ExpectedOption {
                actual: actual.type_info()?,
            }),
        }
    }

    /// Try to coerce value into a string.
    #[inline]
    pub fn into_string(self) -> Result<Shared<String>, ValueError> {
        match self {
            Self::String(string) => Ok(string),
            actual => Err(ValueError::ExpectedString {
                actual: actual.type_info()?,
            }),
        }
    }

    /// Try to coerce value into bytes.
    #[inline]
    pub fn into_bytes(self) -> Result<Shared<Bytes>, ValueError> {
        match self {
            Self::Bytes(bytes) => Ok(bytes),
            actual => Err(ValueError::ExpectedBytes {
                actual: actual.type_info()?,
            }),
        }
    }

    /// Try to coerce value into a vector.
    #[inline]
    pub fn into_vec(self) -> Result<Shared<Vec<Value>>, ValueError> {
        match self {
            Self::Vec(vec) => Ok(vec),
            actual => Err(ValueError::ExpectedVec {
                actual: actual.type_info()?,
            }),
        }
    }

    /// Try to coerce value into a tuple.
    #[inline]
    pub fn into_tuple(self) -> Result<Shared<Box<[Value]>>, ValueError> {
        match self {
            Self::Tuple(tuple) => Ok(tuple),
            actual => Err(ValueError::ExpectedTuple {
                actual: actual.type_info()?,
            }),
        }
    }

    /// Try to coerce value into an object.
    #[inline]
    pub fn into_object(self) -> Result<Shared<Object<Value>>, ValueError> {
        match self {
            Self::Object(object) => Ok(object),
            actual => Err(ValueError::ExpectedObject {
                actual: actual.type_info()?,
            }),
        }
    }

    /// Try to coerce value into an external.
    #[inline]
    pub fn into_external(self) -> Result<Shared<Any>, ValueError> {
        match self {
            Self::External(any) => Ok(any),
            actual => Err(ValueError::ExpectedExternal {
                actual: actual.type_info()?,
            }),
        }
    }

    /// Try to coerce value into an external ref and an associated guard.
    ///
    /// # Safety
    ///
    /// This coerces a strong guard to the value into its raw components.
    ///
    /// It is up to the caller to ensure that the returned pointer does not
    /// outlive the returned guard, not the virtual machine the value belongs
    /// to.
    #[inline]
    pub unsafe fn unsafe_into_external_ref<T>(self) -> Result<(*const T, RawOwnedRef), ValueError>
    where
        T: any::Any,
    {
        match self {
            Self::External(external) => {
                let external = external.downcast_owned_ref::<T>()?;
                let (data, guard) = OwnedRef::into_raw(external);
                Ok((data, guard.into()))
            }
            Self::Ptr(ptr) => {
                let ptr = ptr.downcast_owned_ref::<T>()?;
                let (data, guard) = OwnedRef::into_raw(ptr);
                Ok((data, guard.into()))
            }
            actual => Err(ValueError::ExpectedExternal {
                actual: actual.type_info()?,
            }),
        }
    }

    /// Try to coerce value into an external ref and an associated guard.
    ///
    /// # Safety
    ///
    /// This coerces a strong guard to the value into its raw components.
    ///
    /// It is up to the caller to ensure that the returned pointer does not
    /// outlive the returned guard, not the virtual machine the value belongs
    /// to.
    #[inline]
    pub unsafe fn unsafe_into_external_mut<T>(self) -> Result<(*mut T, RawOwnedMut), ValueError>
    where
        T: any::Any,
    {
        match self {
            Self::External(external) => {
                let external = external.downcast_owned_mut::<T>()?;
                let (data, guard) = OwnedMut::into_raw(external);
                Ok((data, guard.into()))
            }
            Self::Ptr(ptr) => {
                let ptr = ptr.downcast_owned_mut::<T>()?;
                let (data, guard) = OwnedMut::into_raw(ptr);
                Ok((data, guard.into()))
            }
            actual => Err(ValueError::ExpectedExternal {
                actual: actual.type_info()?,
            }),
        }
    }

    /// Get the type information for the current value.
    pub fn value_type(&self) -> Result<ValueType, ValueError> {
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
            Self::TypedObject(object) => ValueType::TypedObject {
                hash: object.borrow_ref()?.hash,
            },
            Self::VariantObject(object) => {
                let object = object.borrow_ref()?;
                ValueType::VariantObject {
                    enum_hash: object.enum_hash,
                    hash: object.hash,
                }
            }
            Self::TypedTuple(tuple) => ValueType::TypedTuple {
                hash: tuple.borrow_ref()?.hash,
            },
            Self::VariantTuple(tuple) => {
                let tuple = tuple.borrow_ref()?;
                ValueType::VariantTuple {
                    enum_hash: tuple.enum_hash,
                    hash: tuple.hash,
                }
            }
            Self::External(any) => ValueType::External(any.borrow_ref()?.type_id()),
            Self::Ptr(ptr) => ValueType::External(ptr.borrow_ref()?.type_id()),
        })
    }

    /// Get the type information for the current value.
    pub fn type_info(&self) -> Result<ValueTypeInfo, ValueError> {
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
            Self::TypedObject(object) => object.borrow_ref()?.type_info(),
            Self::VariantObject(object) => object.borrow_ref()?.type_info(),
            Self::TypedTuple(tuple) => tuple.borrow_ref()?.type_info(),
            Self::VariantTuple(tuple) => tuple.borrow_ref()?.type_info(),
            Self::External(external) => ValueTypeInfo::External(external.borrow_ref()?.type_name()),
            Self::Ptr(ptr) => ValueTypeInfo::External(ptr.borrow_ref()?.type_name()),
        })
    }
}

/// A type-erased rust number.
#[derive(Debug, Clone, Copy)]
pub enum Integer {
    /// `u8`
    U8(u8),
    /// `u16`
    U16(u16),
    /// `u32`
    U32(u32),
    /// `u64`
    U64(u64),
    /// `u128`
    U128(u128),
    /// `i8`
    I8(i8),
    /// `i16`
    I16(i16),
    /// `i32`
    I32(i32),
    /// `i64`
    I64(i64),
    /// `i128`
    I128(i128),
    /// `isize`
    Isize(isize),
    /// `usize`
    Usize(usize),
}

impl fmt::Display for Integer {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::U8(n) => write!(fmt, "{}u8", n),
            Self::U16(n) => write!(fmt, "{}u16", n),
            Self::U32(n) => write!(fmt, "{}u32", n),
            Self::U64(n) => write!(fmt, "{}u64", n),
            Self::U128(n) => write!(fmt, "{}u128", n),
            Self::I8(n) => write!(fmt, "{}i8", n),
            Self::I16(n) => write!(fmt, "{}i16", n),
            Self::I32(n) => write!(fmt, "{}i32", n),
            Self::I64(n) => write!(fmt, "{}i64", n),
            Self::I128(n) => write!(fmt, "{}i128", n),
            Self::Isize(n) => write!(fmt, "{}isize", n),
            Self::Usize(n) => write!(fmt, "{}usize", n),
        }
    }
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
