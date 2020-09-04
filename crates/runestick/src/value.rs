use crate::{
    Any, Bytes, Function, Future, Generator, GeneratorState, Hash, OwnedMut, OwnedRef, RawOwnedMut,
    RawOwnedRef, Shared, StaticString, Stream, Tuple, ValueType, ValueTypeInfo, VmError,
};
use std::any;
use std::fmt;
use std::sync::Arc;

/// The type of an object.
pub type Object<T> = crate::collections::HashMap<String, T>;

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
    StaticString(Arc<StaticString>),
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
    /// A Stream.
    Stream(Shared<Stream>),
    /// A stored generator.
    Generator(Shared<Generator>),
    /// Generator state.
    GeneratorState(Shared<GeneratorState>),
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
    Function(Shared<Function>),
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
    pub fn into_unit(self) -> Result<(), VmError> {
        match self {
            Value::Unit => Ok(()),
            actual => Err(VmError::expected::<()>(actual.type_info()?)),
        }
    }

    /// Try to coerce value into a boolean.
    #[inline]
    pub fn into_bool(self) -> Result<bool, VmError> {
        match self {
            Self::Bool(b) => Ok(b),
            actual => Err(VmError::expected::<bool>(actual.type_info()?)),
        }
    }

    /// Try to coerce value into a byte.
    #[inline]
    pub fn into_byte(self) -> Result<u8, VmError> {
        match self {
            Self::Byte(b) => Ok(b),
            actual => Err(VmError::expected::<u8>(actual.type_info()?)),
        }
    }

    /// Try to coerce value into a character.
    #[inline]
    pub fn into_char(self) -> Result<char, VmError> {
        match self {
            Self::Char(c) => Ok(c),
            actual => Err(VmError::expected::<char>(actual.type_info()?)),
        }
    }

    /// Try to coerce value into an integer.
    #[inline]
    pub fn into_integer(self) -> Result<i64, VmError> {
        match self {
            Self::Integer(integer) => Ok(integer),
            actual => Err(VmError::expected::<i64>(actual.type_info()?)),
        }
    }

    /// Try to coerce value into a float.
    #[inline]
    pub fn into_float(self) -> Result<f64, VmError> {
        match self {
            Self::Float(float) => Ok(float),
            actual => Err(VmError::expected::<f64>(actual.type_info()?)),
        }
    }

    /// Try to coerce value into a result.
    #[inline]
    pub fn into_result(self) -> Result<Shared<Result<Value, Value>>, VmError> {
        match self {
            Self::Result(result) => Ok(result),
            actual => Err(VmError::expected::<Result<Value, Value>>(
                actual.type_info()?,
            )),
        }
    }

    /// Try to coerce value into a future.
    #[inline]
    pub fn into_future(self) -> Result<Shared<Future>, VmError> {
        match self {
            Value::Future(future) => Ok(future),
            actual => Err(VmError::expected::<Future>(actual.type_info()?)),
        }
    }

    /// Try to coerce value into a generator.
    #[inline]
    pub fn into_generator(self) -> Result<Shared<Generator>, VmError> {
        match self {
            Value::Generator(generator) => Ok(generator),
            actual => Err(VmError::expected::<Generator>(actual.type_info()?)),
        }
    }

    /// Try to coerce value into a stream.
    #[inline]
    pub fn into_stream(self) -> Result<Shared<Stream>, VmError> {
        match self {
            Value::Stream(stream) => Ok(stream),
            actual => Err(VmError::expected::<Stream>(actual.type_info()?)),
        }
    }

    /// Try to coerce value into a future.
    #[inline]
    pub fn into_generator_state(self) -> Result<Shared<GeneratorState>, VmError> {
        match self {
            Value::GeneratorState(state) => Ok(state),
            actual => Err(VmError::expected::<GeneratorState>(actual.type_info()?)),
        }
    }

    /// Try to coerce value into an option.
    #[inline]
    pub fn into_option(self) -> Result<Shared<Option<Value>>, VmError> {
        match self {
            Self::Option(option) => Ok(option),
            actual => Err(VmError::expected::<Option<Value>>(actual.type_info()?)),
        }
    }

    /// Try to coerce value into a string.
    #[inline]
    pub fn into_string(self) -> Result<Shared<String>, VmError> {
        match self {
            Self::String(string) => Ok(string),
            actual => Err(VmError::expected::<String>(actual.type_info()?)),
        }
    }

    /// Try to coerce value into bytes.
    #[inline]
    pub fn into_bytes(self) -> Result<Shared<Bytes>, VmError> {
        match self {
            Self::Bytes(bytes) => Ok(bytes),
            actual => Err(VmError::expected::<Bytes>(actual.type_info()?)),
        }
    }

    /// Try to coerce value into a vector.
    #[inline]
    pub fn into_vec(self) -> Result<Shared<Vec<Value>>, VmError> {
        match self {
            Self::Vec(vec) => Ok(vec),
            actual => Err(VmError::expected::<Vec<Value>>(actual.type_info()?)),
        }
    }

    /// Try to coerce value into a tuple.
    #[inline]
    pub fn into_tuple(self) -> Result<Shared<Tuple>, VmError> {
        match self {
            Self::Tuple(tuple) => Ok(tuple),
            actual => Err(VmError::expected::<Tuple>(actual.type_info()?)),
        }
    }

    /// Try to coerce value into an object.
    #[inline]
    pub fn into_object(self) -> Result<Shared<Object<Value>>, VmError> {
        match self {
            Self::Object(object) => Ok(object),
            actual => Err(VmError::expected::<Object<Value>>(actual.type_info()?)),
        }
    }

    /// Try to coerce value into a function pointer.
    #[inline]
    pub fn into_function(self) -> Result<Shared<Function>, VmError> {
        match self {
            Self::Function(function) => Ok(function),
            actual => Err(VmError::expected::<Function>(actual.type_info()?)),
        }
    }

    /// Try to coerce value into an opaque value.
    #[inline]
    pub fn into_any(self) -> Result<Shared<Any>, VmError> {
        match self {
            Self::Any(any) => Ok(any),
            actual => Err(VmError::expected_any(actual.type_info()?)),
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
    pub unsafe fn unsafe_into_any_ref<T>(self) -> Result<(*const T, RawOwnedRef), VmError>
    where
        T: any::Any,
    {
        match self {
            Self::Any(any) => {
                let any = any.downcast_owned_ref::<T>()?;
                let (data, guard) = OwnedRef::into_raw(any);
                Ok((data, guard))
            }
            actual => Err(VmError::expected_any(actual.type_info()?)),
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
    pub unsafe fn unsafe_into_any_mut<T>(self) -> Result<(*mut T, RawOwnedMut), VmError>
    where
        T: any::Any,
    {
        match self {
            Self::Any(any) => {
                let any = any.downcast_owned_mut::<T>()?;
                let (data, guard) = OwnedMut::into_raw(any);
                Ok((data, guard))
            }
            actual => Err(VmError::expected_any(actual.type_info()?)),
        }
    }

    /// Get the type information for the current value.
    pub fn value_type(&self) -> Result<ValueType, VmError> {
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
            Self::Stream(..) => ValueType::StaticType(crate::STREAM_TYPE),
            Self::Generator(..) => ValueType::StaticType(crate::GENERATOR_TYPE),
            Self::GeneratorState(..) => ValueType::StaticType(crate::GENERATOR_STATE_TYPE),
            Self::Result(..) => ValueType::StaticType(crate::RESULT_TYPE),
            Self::Option(..) => ValueType::StaticType(crate::OPTION_TYPE),
            Self::Function(..) => ValueType::StaticType(crate::FUNCTION_TYPE),
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
    pub fn type_info(&self) -> Result<ValueTypeInfo, VmError> {
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
            Self::Stream(..) => ValueTypeInfo::StaticType(crate::STREAM_TYPE),
            Self::Generator(..) => ValueTypeInfo::StaticType(crate::GENERATOR_TYPE),
            Self::GeneratorState(..) => ValueTypeInfo::StaticType(crate::GENERATOR_STATE_TYPE),
            Self::Option(..) => ValueTypeInfo::StaticType(crate::OPTION_TYPE),
            Self::Result(..) => ValueTypeInfo::StaticType(crate::RESULT_TYPE),
            Self::Function(..) => ValueTypeInfo::StaticType(crate::FUNCTION_TYPE),
            Self::Type(hash) => ValueTypeInfo::Type(*hash),
            Self::TypedObject(object) => object.borrow_ref()?.type_info(),
            Self::VariantObject(object) => object.borrow_ref()?.type_info(),
            Self::TypedTuple(tuple) => tuple.borrow_ref()?.type_info(),
            Self::VariantTuple(tuple) => tuple.borrow_ref()?.type_info(),
            Self::Any(any) => ValueTypeInfo::Any(any.borrow_ref()?.type_name()),
        })
    }

    /// Optimized function to test if two value pointers are deeply equal to
    /// each other.
    ///
    /// This is the basis for the eq operation (`==`).
    pub(crate) fn value_ptr_eq(a: &Value, b: &Value) -> Result<bool, VmError> {
        Ok(match (a, b) {
            (Self::Unit, Self::Unit) => true,
            (Self::Char(a), Self::Char(b)) => a == b,
            (Self::Bool(a), Self::Bool(b)) => a == b,
            (Self::Integer(a), Self::Integer(b)) => a == b,
            (Self::Float(a), Self::Float(b)) => a == b,
            (Self::Vec(a), Self::Vec(b)) => {
                let a = a.borrow_ref()?;
                let b = b.borrow_ref()?;

                if a.len() != b.len() {
                    return Ok(false);
                }

                for (a, b) in a.iter().zip(b.iter()) {
                    if !Self::value_ptr_eq(a, b)? {
                        return Ok(false);
                    }
                }

                true
            }
            (Self::Object(a), Self::Object(b)) => {
                let a = a.borrow_ref()?;
                let b = b.borrow_ref()?;

                if a.len() != b.len() {
                    return Ok(false);
                }

                for (key, a) in a.iter() {
                    let b = match b.get(key) {
                        Some(b) => b,
                        None => return Ok(false),
                    };

                    if !Self::value_ptr_eq(a, b)? {
                        return Ok(false);
                    }
                }

                true
            }
            (Self::String(a), Self::String(b)) => {
                let a = a.borrow_ref()?;
                let b = b.borrow_ref()?;
                *a == *b
            }
            (Self::StaticString(a), Self::String(b)) => {
                let b = b.borrow_ref()?;
                ***a == *b
            }
            (Self::String(a), Self::StaticString(b)) => {
                let a = a.borrow_ref()?;
                *a == ***b
            }
            // fast string comparison: exact string slot.
            (Self::StaticString(a), Self::StaticString(b)) => ***a == ***b,
            // fast external comparison by slot.
            // TODO: implement ptr equals.
            // (Self::Any(a), Self::Any(b)) => a == b,
            _ => false,
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
                write!(f, "Type({})", value)?;
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
            Value::Stream(value) => {
                write!(f, "{:?}", value)?;
            }
            Value::Generator(value) => {
                write!(f, "{:?}", value)?;
            }
            Value::GeneratorState(value) => {
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
            Value::Function(value) => {
                write!(f, "{:?}", value)?;
            }
            Value::Any(value) => {
                write!(f, "{:?}", value)?;
            }
        }

        Ok(())
    }
}

impl From<()> for Value {
    fn from((): ()) -> Self {
        Self::Unit
    }
}

macro_rules! impl_from {
    ($ty:ty, $variant:ident) => {
        impl From<$ty> for Value {
            fn from(value: $ty) -> Self {
                Self::$variant(value)
            }
        }
    };
}

impl_from!(u8, Byte);
impl_from!(bool, Bool);
impl_from!(char, Char);
impl_from!(i64, Integer);
impl_from!(f64, Float);
impl_from!(Arc<StaticString>, StaticString);

macro_rules! impl_from_shared {
    (Shared<$ty:ty>, $variant:ident) => {
        impl_from!(Shared<$ty>, $variant);

        impl From<$ty> for Value {
            fn from(value: $ty) -> Self {
                Self::$variant(Shared::new(value))
            }
        }
    };
}

impl_from_shared!(Shared<Bytes>, Bytes);
impl_from_shared!(Shared<String>, String);
impl_from!(Shared<Vec<Value>>, Vec);
impl_from_shared!(Shared<Tuple>, Tuple);
impl_from!(Shared<Object<Value>>, Object);
impl_from_shared!(Shared<Future>, Future);
impl_from_shared!(Shared<Stream>, Stream);
impl_from_shared!(Shared<Generator>, Generator);
impl_from_shared!(Shared<GeneratorState>, GeneratorState);
impl_from!(Shared<Option<Value>>, Option);
impl_from!(Shared<Result<Value, Value>>, Result);
impl_from_shared!(Shared<TypedTuple>, TypedTuple);
impl_from_shared!(Shared<VariantTuple>, VariantTuple);
impl_from_shared!(Shared<TypedObject>, TypedObject);
impl_from_shared!(Shared<VariantObject>, VariantObject);
impl_from_shared!(Shared<Function>, Function);
impl_from_shared!(Shared<Any>, Any);

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
