use crate::access::AccessKind;
use crate::{
    Any, AnyObj, Bytes, Function, Future, Generator, GeneratorState, Hash, Mut, Object, RawMut,
    RawRef, Ref, Shared, StaticString, Stream, Tuple, Type, TypeInfo, VmError,
};
use std::fmt;
use std::sync::Arc;

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
    pub fn type_info(&self) -> TypeInfo {
        TypeInfo::Hash(self.hash)
    }

    /// Get the value at the given index in the tuple.
    pub fn get(&self, index: usize) -> Option<&Value> {
        self.tuple.get(index)
    }

    /// Get the mutable value at the given index in the tuple.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut Value> {
        self.tuple.get_mut(index)
    }
}

/// A tuple with a well-defined type as a variant of an enum.
#[derive(Debug)]
pub struct TupleVariant {
    /// The type hash of the enum.
    pub(crate) enum_hash: Hash,
    /// The variant type hash of the tuple.
    pub(crate) hash: Hash,
    /// Content of the tuple.
    pub(crate) tuple: Box<[Value]>,
}

impl TupleVariant {
    /// Get type info for the typed tuple.
    pub fn type_info(&self) -> TypeInfo {
        TypeInfo::Hash(self.enum_hash)
    }
}

/// An object with a well-defined type.
#[derive(Debug)]
pub struct TypedObject {
    /// The type hash of the object.
    hash: Hash,
    /// Content of the object.
    pub(crate) object: Object,
}

impl TypedObject {
    /// Construct a new typed object with the given type hash.
    pub fn new(hash: Hash, object: Object) -> Self {
        Self { hash, object }
    }

    /// Get type info for the typed object.
    pub fn type_info(&self) -> TypeInfo {
        TypeInfo::Hash(self.hash)
    }

    /// Get the type hash of the object.
    #[inline]
    pub fn type_hash(&self) -> Hash {
        self.hash
    }

    /// Get the given key in the object.
    pub fn get<Q: ?Sized>(&self, k: &Q) -> Option<&Value>
    where
        String: std::borrow::Borrow<Q>,
        Q: std::hash::Hash + std::cmp::Eq,
    {
        self.object.get(k)
    }

    /// Get the given mutable value by key in the object.
    pub fn get_mut<Q: ?Sized>(&mut self, k: &Q) -> Option<&mut Value>
    where
        String: std::borrow::Borrow<Q>,
        Q: std::hash::Hash + std::cmp::Eq,
    {
        self.object.get_mut(k)
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
    pub object: Object,
}

impl VariantObject {
    /// Get type info for the typed object.
    pub fn type_info(&self) -> TypeInfo {
        TypeInfo::Hash(self.enum_hash)
    }

    /// Get the given key in the object.
    pub fn get<Q: ?Sized>(&self, k: &Q) -> Option<&Value>
    where
        String: std::borrow::Borrow<Q>,
        Q: std::hash::Hash + std::cmp::Eq,
    {
        self.object.get(k)
    }

    /// Get the given mutable value by key in the object.
    pub fn get_mut<Q: ?Sized>(&mut self, k: &Q) -> Option<&mut Value>
    where
        String: std::borrow::Borrow<Q>,
        Q: std::hash::Hash + std::cmp::Eq,
    {
        self.object.get_mut(k)
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
    /// A type hash. Describes a type in the virtual machine.
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
    Object(Shared<Object>),
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
    TupleVariant(Shared<TupleVariant>),
    /// An object with a well-defined type.
    TypedObject(Shared<TypedObject>),
    /// An object variant with a well-defined type.
    VariantObject(Shared<VariantObject>),
    /// A stored function pointer.
    Function(Shared<Function>),
    /// An opaque value that can be downcasted.
    Any(Shared<AnyObj>),
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
        Self::TupleVariant(Shared::new(TupleVariant {
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
    pub fn into_object(self) -> Result<Shared<Object>, VmError> {
        match self {
            Self::Object(object) => Ok(object),
            actual => Err(VmError::expected::<Object>(actual.type_info()?)),
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
    pub fn into_any(self) -> Result<Shared<AnyObj>, VmError> {
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
    pub unsafe fn unsafe_into_any_ref<T>(self) -> Result<(*const T, RawRef), VmError>
    where
        T: Any,
    {
        match self {
            Self::Any(any) => {
                let any = any.internal_downcast_into_ref::<T>(AccessKind::Any)?;
                let (data, guard) = Ref::into_raw(any);
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
    pub unsafe fn unsafe_into_any_mut<T>(self) -> Result<(*mut T, RawMut), VmError>
    where
        T: Any,
    {
        match self {
            Self::Any(any) => {
                let any = any.internal_downcast_into_mut::<T>(AccessKind::Any)?;
                let (data, guard) = Mut::into_raw(any);
                Ok((data, guard))
            }
            actual => Err(VmError::expected_any(actual.type_info()?)),
        }
    }

    /// Get the type information for the current value.
    pub fn type_of(&self) -> Result<Type, VmError> {
        Ok(match self {
            Self::Unit => Type::from(crate::UNIT_TYPE),
            Self::Bool(..) => Type::from(crate::BOOL_TYPE),
            Self::Byte(..) => Type::from(crate::BYTE_TYPE),
            Self::Char(..) => Type::from(crate::CHAR_TYPE),
            Self::Integer(..) => Type::from(crate::INTEGER_TYPE),
            Self::Float(..) => Type::from(crate::FLOAT_TYPE),
            Self::StaticString(..) => Type::from(crate::STRING_TYPE),
            Self::String(..) => Type::from(crate::STRING_TYPE),
            Self::Bytes(..) => Type::from(crate::BYTES_TYPE),
            Self::Vec(..) => Type::from(crate::VEC_TYPE),
            Self::Tuple(..) => Type::from(crate::TUPLE_TYPE),
            Self::Object(..) => Type::from(crate::OBJECT_TYPE),
            Self::Future(..) => Type::from(crate::FUTURE_TYPE),
            Self::Stream(..) => Type::from(crate::STREAM_TYPE),
            Self::Generator(..) => Type::from(crate::GENERATOR_TYPE),
            Self::GeneratorState(..) => Type::from(crate::GENERATOR_STATE_TYPE),
            Self::Result(..) => Type::from(crate::RESULT_TYPE),
            Self::Option(..) => Type::from(crate::OPTION_TYPE),
            Self::Function(..) => Type::from(crate::FUNCTION_TYPE),
            Self::Type(hash) => Type::from(*hash),
            Self::TypedObject(object) => Type::from(object.borrow_ref()?.hash),
            Self::VariantObject(object) => {
                let object = object.borrow_ref()?;
                Type::from(object.enum_hash)
            }
            Self::TypedTuple(tuple) => Type::from(tuple.borrow_ref()?.hash),
            Self::TupleVariant(tuple) => {
                let tuple = tuple.borrow_ref()?;
                Type::from(tuple.enum_hash)
            }
            Self::Any(any) => Type::from(any.borrow_ref()?.type_hash()),
        })
    }

    /// Get the type information for the current value.
    pub fn type_info(&self) -> Result<TypeInfo, VmError> {
        Ok(match self {
            Self::Unit => TypeInfo::StaticType(crate::UNIT_TYPE),
            Self::Bool(..) => TypeInfo::StaticType(crate::BOOL_TYPE),
            Self::Byte(..) => TypeInfo::StaticType(crate::BYTE_TYPE),
            Self::Char(..) => TypeInfo::StaticType(crate::CHAR_TYPE),
            Self::Integer(..) => TypeInfo::StaticType(crate::INTEGER_TYPE),
            Self::Float(..) => TypeInfo::StaticType(crate::FLOAT_TYPE),
            Self::StaticString(..) => TypeInfo::StaticType(crate::STRING_TYPE),
            Self::String(..) => TypeInfo::StaticType(crate::STRING_TYPE),
            Self::Bytes(..) => TypeInfo::StaticType(crate::BYTES_TYPE),
            Self::Vec(..) => TypeInfo::StaticType(crate::VEC_TYPE),
            Self::Tuple(..) => TypeInfo::StaticType(crate::TUPLE_TYPE),
            Self::Object(..) => TypeInfo::StaticType(crate::OBJECT_TYPE),
            Self::Future(..) => TypeInfo::StaticType(crate::FUTURE_TYPE),
            Self::Stream(..) => TypeInfo::StaticType(crate::STREAM_TYPE),
            Self::Generator(..) => TypeInfo::StaticType(crate::GENERATOR_TYPE),
            Self::GeneratorState(..) => TypeInfo::StaticType(crate::GENERATOR_STATE_TYPE),
            Self::Option(..) => TypeInfo::StaticType(crate::OPTION_TYPE),
            Self::Result(..) => TypeInfo::StaticType(crate::RESULT_TYPE),
            Self::Function(..) => TypeInfo::StaticType(crate::FUNCTION_TYPE),
            Self::Type(hash) => TypeInfo::Hash(*hash),
            Self::TypedObject(object) => object.borrow_ref()?.type_info(),
            Self::VariantObject(object) => object.borrow_ref()?.type_info(),
            Self::TypedTuple(tuple) => tuple.borrow_ref()?.type_info(),
            Self::TupleVariant(tuple) => tuple.borrow_ref()?.type_info(),
            Self::Any(any) => TypeInfo::Any(any.borrow_ref()?.type_name()),
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
            Value::TupleVariant(value) => {
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

impl<T> From<T> for Value
where
    T: Any,
{
    fn from(any: T) -> Self {
        Self::Any(Shared::new(AnyObj::new(any)))
    }
}

impl From<()> for Value {
    fn from((): ()) -> Self {
        Self::Unit
    }
}

impl crate::ToValue for Value {
    fn to_value(self) -> Result<Value, VmError> {
        Ok(self)
    }
}

impl crate::ToValue for () {
    fn to_value(self) -> Result<Value, VmError> {
        Ok(Value::from(self))
    }
}

macro_rules! impl_from {
    ($ty:ty, $variant:ident) => {
        impl From<$ty> for Value {
            fn from(value: $ty) -> Self {
                Self::$variant(value)
            }
        }

        impl $crate::ToValue for $ty {
            fn to_value(self) -> Result<Value, VmError> {
                Ok(Value::from(self))
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

        impl $crate::ToValue for $ty {
            fn to_value(self) -> Result<Value, VmError> {
                Ok(Value::from(self))
            }
        }
    };
}

impl_from_shared!(Shared<Bytes>, Bytes);
impl_from_shared!(Shared<String>, String);
impl_from!(Shared<Vec<Value>>, Vec);
impl_from_shared!(Shared<Tuple>, Tuple);
impl_from_shared!(Shared<Object>, Object);
impl_from_shared!(Shared<Future>, Future);
impl_from_shared!(Shared<Stream>, Stream);
impl_from_shared!(Shared<Generator>, Generator);
impl_from_shared!(Shared<GeneratorState>, GeneratorState);
impl_from!(Shared<Option<Value>>, Option);
impl_from!(Shared<Result<Value, Value>>, Result);
impl_from_shared!(Shared<TypedTuple>, TypedTuple);
impl_from_shared!(Shared<TupleVariant>, TupleVariant);
impl_from_shared!(Shared<TypedObject>, TypedObject);
impl_from_shared!(Shared<VariantObject>, VariantObject);
impl_from_shared!(Shared<Function>, Function);
impl_from_shared!(Shared<AnyObj>, Any);

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
