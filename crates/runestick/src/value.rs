use crate::{
    AccessError, Any, Bytes, FnPtr, Future, Hash, OwnedMut, OwnedRef, Panic, RawOwnedMut,
    RawOwnedRef, Shared, StaticString, Tuple, ValueType, ValueTypeInfo, VmError,
};
use std::any;
use std::fmt;
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
    /// A wrapped virtual machine error.
    #[error("{error}")]
    VmError {
        /// The source error.
        #[source]
        error: Box<VmError>,
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
    /// Error raised when we expected a function pointer.
    #[error("expected a function pointer but found `{actual}`")]
    ExpectedFnPtr {
        /// The actual type observed instead.
        actual: ValueTypeInfo,
    },
    /// Error raised when we expected a value.
    #[error("expected a value of `{expected}` but found `{actual}`")]
    ExpectedAny {
        /// Expected type.
        expected: &'static str,
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
    pub(crate) hash: Hash,
    /// Content of the tuple.
    pub(crate) tuple: Box<[Value]>,
}

impl TypedTuple {
    /// Get type info for the typed tuple.
    pub fn type_info(&self) -> ValueTypeInfo {
        ValueTypeInfo::Type(self.hash)
    }
}

/// A tuple with a well-defined type as a variant of an enum.
#[derive(Debug)]
pub struct VariantTuple {
    /// The type hash of the enum.
    pub(crate) enum_hash: Hash,
    /// The variant type hash of the tuple.
    pub(crate) hash: Hash,
    /// Content of the tuple.
    pub(crate) tuple: Box<[Value]>,
}

impl VariantTuple {
    /// Get type info for the typed tuple.
    pub fn type_info(&self) -> ValueTypeInfo {
        ValueTypeInfo::Type(self.enum_hash)
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
        ValueTypeInfo::Type(self.hash)
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
        ValueTypeInfo::Type(self.enum_hash)
    }
}

/// An entry on the stack.
#[derive(Clone)]
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
    StaticString(StaticString),
    /// A UTF-8 string.
    String(Shared<String>),
    /// A byte string.
    Bytes(Shared<Bytes>),
    /// A vector containing any values.
    Vec(Shared<Vec<Value>>),
    /// A tuple.
    Tuple(Shared<Tuple>),
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
    /// A stored function pointer.
    FnPtr(Shared<FnPtr>),
    /// An opaque value that can be downcasted.
    Any(Shared<Any>),
}

impl Value {
    /// Construct a vector.
    pub fn vec(vec: Vec<Value>) -> Self {
        Self::Vec(Shared::new(vec))
    }

    /// Construct a tuple.
    pub fn tuple(vec: Vec<Value>) -> Self {
        Self::Tuple(Shared::new(Tuple::from(vec)))
    }

    /// Construct a typed tuple.
    pub fn typed_tuple(hash: Hash, vec: Vec<Value>) -> Self {
        Self::TypedTuple(Shared::new(TypedTuple {
            hash,
            tuple: vec.into_boxed_slice(),
        }))
    }

    /// Construct a typed tuple.
    pub fn variant_tuple(enum_hash: Hash, hash: Hash, vec: Vec<Value>) -> Self {
        Self::VariantTuple(Shared::new(VariantTuple {
            enum_hash,
            hash,
            tuple: vec.into_boxed_slice(),
        }))
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
    pub fn into_tuple(self) -> Result<Shared<Tuple>, ValueError> {
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

    /// Try to coerce value into a function pointer.
    #[inline]
    pub fn into_fn_ptr(self) -> Result<Shared<FnPtr>, ValueError> {
        match self {
            Self::FnPtr(fn_ptr) => Ok(fn_ptr),
            actual => Err(ValueError::ExpectedFnPtr {
                actual: actual.type_info()?,
            }),
        }
    }

    /// Try to coerce value into an opaque value.
    #[inline]
    pub fn into_any(self) -> Result<Shared<Any>, ValueError> {
        match self {
            Self::Any(any) => Ok(any),
            actual => Err(ValueError::ExpectedAny {
                expected: any::type_name::<Any>(),
                actual: actual.type_info()?,
            }),
        }
    }

    /// Try to coerce value into a ref and an associated guard.
    ///
    /// # Safety
    ///
    /// This coerces a strong guard to the value into its raw components.
    ///
    /// It is up to the caller to ensure that the returned pointer does not
    /// outlive the returned guard, not the virtual machine the value belongs
    /// to.
    #[inline]
    pub unsafe fn unsafe_into_any_ref<T>(self) -> Result<(*const T, RawOwnedRef), ValueError>
    where
        T: any::Any,
    {
        match self {
            Self::Any(any) => {
                let any = any.downcast_owned_ref::<T>()?;
                let (data, guard) = OwnedRef::into_raw(any);
                Ok((data, guard))
            }
            actual => Err(ValueError::ExpectedAny {
                expected: any::type_name::<T>(),
                actual: actual.type_info()?,
            }),
        }
    }

    /// Try to coerce value into a ref and an associated guard.
    ///
    /// # Safety
    ///
    /// This coerces a strong guard to the value into its raw components.
    ///
    /// It is up to the caller to ensure that the returned pointer does not
    /// outlive the returned guard, not the virtual machine the value belongs
    /// to.
    #[inline]
    pub unsafe fn unsafe_into_any_mut<T>(self) -> Result<(*mut T, RawOwnedMut), ValueError>
    where
        T: any::Any,
    {
        match self {
            Self::Any(any) => {
                let any = any.downcast_owned_mut::<T>()?;
                let (data, guard) = OwnedMut::into_raw(any);
                Ok((data, guard))
            }
            actual => Err(ValueError::ExpectedAny {
                expected: any::type_name::<T>(),
                actual: actual.type_info()?,
            }),
        }
    }

    /// Get the type information for the current value.
    pub fn value_type(&self) -> Result<ValueType, ValueError> {
        Ok(match self {
            Self::Unit => ValueType::StaticType(crate::UNIT_TYPE),
            Self::Bool(..) => ValueType::StaticType(crate::BOOL_TYPE),
            Self::Byte(..) => ValueType::StaticType(crate::BYTE_TYPE),
            Self::Char(..) => ValueType::StaticType(crate::CHAR_TYPE),
            Self::Integer(..) => ValueType::StaticType(crate::INTEGER_TYPE),
            Self::Float(..) => ValueType::StaticType(crate::FLOAT_TYPE),
            Self::StaticString(..) => ValueType::StaticType(crate::STRING_TYPE),
            Self::String(..) => ValueType::StaticType(crate::STRING_TYPE),
            Self::Bytes(..) => ValueType::StaticType(crate::BYTES_TYPE),
            Self::Vec(..) => ValueType::StaticType(crate::VEC_TYPE),
            Self::Tuple(..) => ValueType::StaticType(crate::TUPLE_TYPE),
            Self::Object(..) => ValueType::StaticType(crate::OBJECT_TYPE),
            Self::Future(..) => ValueType::StaticType(crate::FUTURE_TYPE),
            Self::Result(..) => ValueType::StaticType(crate::RESULT_TYPE),
            Self::Option(..) => ValueType::StaticType(crate::OPTION_TYPE),
            Self::FnPtr(..) => ValueType::StaticType(crate::FN_PTR_TYPE),
            Self::Type(hash) => ValueType::Type(*hash),
            Self::TypedObject(object) => ValueType::Type(object.borrow_ref()?.hash),
            Self::VariantObject(object) => {
                let object = object.borrow_ref()?;
                ValueType::Type(object.enum_hash)
            }
            Self::TypedTuple(tuple) => ValueType::Type(tuple.borrow_ref()?.hash),
            Self::VariantTuple(tuple) => {
                let tuple = tuple.borrow_ref()?;
                ValueType::Type(tuple.enum_hash)
            }
            Self::Any(any) => ValueType::Type(any.borrow_ref()?.type_hash()),
        })
    }

    /// Get the type information for the current value.
    pub fn type_info(&self) -> Result<ValueTypeInfo, ValueError> {
        Ok(match self {
            Self::Unit => ValueTypeInfo::StaticType(crate::UNIT_TYPE),
            Self::Bool(..) => ValueTypeInfo::StaticType(crate::BOOL_TYPE),
            Self::Byte(..) => ValueTypeInfo::StaticType(crate::BYTE_TYPE),
            Self::Char(..) => ValueTypeInfo::StaticType(crate::CHAR_TYPE),
            Self::Integer(..) => ValueTypeInfo::StaticType(crate::INTEGER_TYPE),
            Self::Float(..) => ValueTypeInfo::StaticType(crate::STRING_TYPE),
            Self::StaticString(..) => ValueTypeInfo::StaticType(crate::STRING_TYPE),
            Self::String(..) => ValueTypeInfo::StaticType(crate::STRING_TYPE),
            Self::Bytes(..) => ValueTypeInfo::StaticType(crate::BYTES_TYPE),
            Self::Vec(..) => ValueTypeInfo::StaticType(crate::VEC_TYPE),
            Self::Tuple(..) => ValueTypeInfo::StaticType(crate::TUPLE_TYPE),
            Self::Object(..) => ValueTypeInfo::StaticType(crate::OBJECT_TYPE),
            Self::Future(..) => ValueTypeInfo::StaticType(crate::FUTURE_TYPE),
            Self::Option(..) => ValueTypeInfo::StaticType(crate::OPTION_TYPE),
            Self::Result(..) => ValueTypeInfo::StaticType(crate::RESULT_TYPE),
            Self::FnPtr(..) => ValueTypeInfo::StaticType(crate::FN_PTR_TYPE),
            Self::Type(hash) => ValueTypeInfo::Type(*hash),
            Self::TypedObject(object) => object.borrow_ref()?.type_info(),
            Self::VariantObject(object) => object.borrow_ref()?.type_info(),
            Self::TypedTuple(tuple) => tuple.borrow_ref()?.type_info(),
            Self::VariantTuple(tuple) => tuple.borrow_ref()?.type_info(),
            Self::Any(any) => ValueTypeInfo::Any(any.borrow_ref()?.type_name()),
        })
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Unit => {
                write!(f, "()")?;
            }
            Value::Bool(value) => {
                write!(f, "{:?}", value)?;
            }
            Value::Byte(value) => {
                write!(f, "{:?}", value)?;
            }
            Value::Char(value) => {
                write!(f, "{:?}", value)?;
            }
            Value::Integer(value) => {
                write!(f, "{:?}", value)?;
            }
            Value::Float(value) => {
                write!(f, "{:?}", value)?;
            }
            Value::Type(value) => {
                write!(f, "{:?}", value)?;
            }
            Value::StaticString(value) => {
                write!(f, "{:?}", value)?;
            }
            Value::String(value) => {
                write!(f, "{:?}", value)?;
            }
            Value::Bytes(value) => {
                write!(f, "{:?}", value)?;
            }
            Value::Vec(value) => {
                write!(f, "{:?}", value)?;
            }
            Value::Tuple(value) => {
                write!(f, "{:?}", value)?;
            }
            Value::Object(value) => {
                write!(f, "{:?}", value)?;
            }
            Value::Future(value) => {
                write!(f, "{:?}", value)?;
            }
            Value::Option(value) => {
                write!(f, "{:?}", value)?;
            }
            Value::Result(value) => {
                write!(f, "{:?}", value)?;
            }
            Value::TypedTuple(value) => {
                write!(f, "{:?}", value)?;
            }
            Value::VariantTuple(value) => {
                write!(f, "{:?}", value)?;
            }
            Value::TypedObject(value) => {
                write!(f, "{:?}", value)?;
            }
            Value::VariantObject(value) => {
                write!(f, "{:?}", value)?;
            }
            Value::FnPtr(value) => {
                write!(f, "{:?}", value)?;
            }
            Value::Any(value) => {
                write!(f, "Any({:?})", value)?;
            }
        }

        Ok(())
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
