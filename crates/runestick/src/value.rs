use crate::access::AccessKind;
use crate::{
    Any, AnyObj, Bytes, Format, Function, Future, Generator, GeneratorState, Hash, Item, Mut,
    Object, RawMut, RawRef, Ref, Shared, StaticString, Stream, Tuple, Type, TypeInfo, Vec, VmError,
};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::Arc;
use std::vec;

/// A empty with a well-defined type.
pub struct UnitStruct {
    /// The type hash of the empty.
    pub(crate) rtti: Arc<Rtti>,
}

impl UnitStruct {
    /// Access runtime type information.
    pub fn rtti(&self) -> &Arc<Rtti> {
        &self.rtti
    }

    /// Get type info for the typed tuple.
    pub fn type_info(&self) -> TypeInfo {
        TypeInfo::Typed(self.rtti.clone())
    }
}

impl fmt::Debug for UnitStruct {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.rtti.item)
    }
}

/// A tuple with a well-defined type.
pub struct TupleStruct {
    /// The type hash of the tuple.
    pub(crate) rtti: Arc<Rtti>,
    /// Content of the tuple.
    pub(crate) data: Tuple,
}

impl TupleStruct {
    /// Access runtime type information.
    pub fn rtti(&self) -> &Arc<Rtti> {
        &self.rtti
    }

    /// Access underlying data.
    pub fn data(&self) -> &Tuple {
        &self.data
    }

    /// Access underlying data mutably.
    pub fn data_mut(&mut self) -> &mut Tuple {
        &mut self.data
    }

    /// Get type info for the typed tuple.
    pub fn type_info(&self) -> TypeInfo {
        TypeInfo::Typed(self.rtti.clone())
    }

    /// Get the value at the given index in the tuple.
    pub fn get(&self, index: usize) -> Option<&Value> {
        self.data.get(index)
    }

    /// Get the mutable value at the given index in the tuple.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut Value> {
        self.data.get_mut(index)
    }
}

impl fmt::Debug for TupleStruct {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{:?}", self.rtti.item, self.data)
    }
}

/// A tuple with a well-defined type as a variant of an enum.
pub struct TupleVariant {
    /// Type information for object variant.
    pub(crate) rtti: Arc<VariantRtti>,
    /// Content of the tuple.
    pub(crate) data: Tuple,
}

impl TupleVariant {
    /// Access runtime type information.
    pub fn rtti(&self) -> &Arc<VariantRtti> {
        &self.rtti
    }

    /// Access underlying data.
    pub fn data(&self) -> &Tuple {
        &self.data
    }

    /// Access underlying data mutably.
    pub fn data_mut(&mut self) -> &mut Tuple {
        &mut self.data
    }

    /// Get type info for the typed tuple.
    pub fn type_info(&self) -> TypeInfo {
        TypeInfo::Variant(self.rtti.clone())
    }

    /// Get the value at the given index in the tuple.
    pub fn get(&self, index: usize) -> Option<&Value> {
        self.data.get(index)
    }

    /// Get the mutable value at the given index in the tuple.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut Value> {
        self.data.get_mut(index)
    }
}

impl fmt::Debug for TupleVariant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{:?}", self.rtti.item, self.data)
    }
}

/// An object with a well-defined type.
pub struct Struct {
    /// The type hash of the object.
    pub(crate) rtti: Arc<Rtti>,
    /// Content of the object.
    pub(crate) data: Object,
}

impl Struct {
    /// Access runtime type information.
    pub fn rtti(&self) -> &Arc<Rtti> {
        &self.rtti
    }

    /// Access underlying data.
    pub fn data(&self) -> &Object {
        &self.data
    }

    /// Access underlying data mutably.
    pub fn data_mut(&mut self) -> &mut Object {
        &mut self.data
    }

    /// Get type info for the typed object.
    pub fn type_info(&self) -> TypeInfo {
        TypeInfo::Typed(self.rtti.clone())
    }

    /// Get the type hash of the object.
    #[inline]
    pub fn type_hash(&self) -> Hash {
        self.rtti.hash
    }

    /// Get the given key in the object.
    pub fn get<Q: ?Sized>(&self, k: &Q) -> Option<&Value>
    where
        String: std::borrow::Borrow<Q>,
        Q: std::hash::Hash + std::cmp::Eq,
    {
        self.data.get(k)
    }

    /// Get the given mutable value by key in the object.
    pub fn get_mut<Q: ?Sized>(&mut self, k: &Q) -> Option<&mut Value>
    where
        String: std::borrow::Borrow<Q>,
        Q: std::hash::Hash + std::cmp::Eq,
    {
        self.data.get_mut(k)
    }
}

impl fmt::Debug for Struct {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{:?}", self.rtti.item, self.data)
    }
}

/// An object with a well-defined variant of an enum.
pub struct StructVariant {
    /// Type information for object variant.
    pub(crate) rtti: Arc<VariantRtti>,
    /// Content of the object.
    pub(crate) data: Object,
}

impl StructVariant {
    /// Access runtime type information.
    pub fn rtti(&self) -> &Arc<VariantRtti> {
        &self.rtti
    }

    /// Access underlying data.
    pub fn data(&self) -> &Object {
        &self.data
    }

    /// Access underlying data mutably.
    pub fn data_mut(&mut self) -> &mut Object {
        &mut self.data
    }

    /// Get type info for the typed object.
    pub fn type_info(&self) -> TypeInfo {
        TypeInfo::Variant(self.rtti.clone())
    }

    /// Get the given key in the object.
    pub fn get<Q: ?Sized>(&self, k: &Q) -> Option<&Value>
    where
        String: std::borrow::Borrow<Q>,
        Q: std::hash::Hash + std::cmp::Eq,
    {
        self.data.get(k)
    }

    /// Get the given mutable value by key in the object.
    pub fn get_mut<Q: ?Sized>(&mut self, k: &Q) -> Option<&mut Value>
    where
        String: std::borrow::Borrow<Q>,
        Q: std::hash::Hash + std::cmp::Eq,
    {
        self.data.get_mut(k)
    }
}

impl fmt::Debug for StructVariant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{:?}", self.rtti.item, self.data)
    }
}

/// An object with a well-defined variant of an enum.
pub struct UnitVariant {
    /// Type information for object variant.
    pub(crate) rtti: Arc<VariantRtti>,
}

impl UnitVariant {
    /// Access runtime type information.
    pub fn rtti(&self) -> &Arc<VariantRtti> {
        &self.rtti
    }

    /// Get type info for the typed object.
    pub fn type_info(&self) -> TypeInfo {
        TypeInfo::Variant(self.rtti.clone())
    }
}

impl fmt::Debug for UnitVariant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.rtti.item)
    }
}

/// Runtime information on variant.
#[derive(Debug, Serialize, Deserialize)]
pub struct VariantRtti {
    /// The type hash of the enum.
    pub enum_hash: Hash,
    /// The type variant hash.
    pub hash: Hash,
    /// The name of the variant.
    pub item: Item,
}

/// Runtime information on variant.
#[derive(Debug, Serialize, Deserialize)]
pub struct Rtti {
    /// The type hash of the type.
    pub hash: Hash,
    /// The item of the type.
    pub item: Item,
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
    Vec(Shared<Vec>),
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
    /// An struct with a well-defined type.
    UnitStruct(Shared<UnitStruct>),
    /// A tuple with a well-defined type.
    TupleStruct(Shared<TupleStruct>),
    /// An struct with a well-defined type.
    Struct(Shared<Struct>),
    /// An struct variant with a well-defined type.
    UnitVariant(Shared<UnitVariant>),
    /// A tuple variant with a well-defined type.
    TupleVariant(Shared<TupleVariant>),
    /// An struct variant with a well-defined type.
    StructVariant(Shared<StructVariant>),
    /// A stored function pointer.
    Function(Shared<Function>),
    /// A value being formatted.
    Format(Box<Format>),
    /// An opaque value that can be downcasted.
    Any(Shared<AnyObj>),
}

impl Value {
    /// Construct a vector.
    pub fn vec(vec: vec::Vec<Value>) -> Self {
        Self::Vec(Shared::new(Vec::from(vec)))
    }

    /// Construct a tuple.
    pub fn tuple(vec: vec::Vec<Value>) -> Self {
        Self::Tuple(Shared::new(Tuple::from(vec)))
    }

    /// Construct an empty.
    pub fn unit_struct(rtti: Arc<Rtti>) -> Self {
        Self::UnitStruct(Shared::new(UnitStruct { rtti }))
    }

    /// Construct a typed tuple.
    pub fn tuple_struct(rtti: Arc<Rtti>, vec: vec::Vec<Value>) -> Self {
        Self::TupleStruct(Shared::new(TupleStruct {
            rtti,
            data: Tuple::from(vec),
        }))
    }

    /// Construct an empty variant.
    pub fn empty_variant(rtti: Arc<VariantRtti>) -> Self {
        Self::UnitVariant(Shared::new(UnitVariant { rtti }))
    }

    /// Construct a tuple variant.
    pub fn tuple_variant(rtti: Arc<VariantRtti>, vec: vec::Vec<Value>) -> Self {
        Self::TupleVariant(Shared::new(TupleVariant {
            rtti,
            data: Tuple::from(vec),
        }))
    }

    /// Take the interior value.
    pub fn take(self) -> Result<Self, VmError> {
        Ok(match self {
            Self::Unit => Self::Unit,
            Self::Bool(value) => Self::Bool(value),
            Self::Byte(value) => Self::Byte(value),
            Self::Char(value) => Self::Char(value),
            Self::Integer(value) => Self::Integer(value),
            Self::Float(value) => Self::Float(value),
            Self::Type(value) => Self::Type(value),
            Self::StaticString(value) => Self::StaticString(value),
            Self::String(value) => Self::String(Shared::new(value.take()?)),
            Self::Bytes(value) => Self::Bytes(Shared::new(value.take()?)),
            Self::Vec(value) => Self::Vec(Shared::new(value.take()?)),
            Self::Tuple(value) => Self::Tuple(Shared::new(value.take()?)),
            Self::Object(value) => Self::Object(Shared::new(value.take()?)),
            Self::Future(value) => Self::Future(Shared::new(value.take()?)),
            Self::Stream(value) => Self::Stream(Shared::new(value.take()?)),
            Self::Generator(value) => Self::Generator(Shared::new(value.take()?)),
            Self::GeneratorState(value) => Self::GeneratorState(Shared::new(value.take()?)),
            Self::Option(value) => Self::Option(Shared::new(value.take()?)),
            Self::Result(value) => Self::Result(Shared::new(value.take()?)),
            Self::UnitStruct(value) => Self::UnitStruct(Shared::new(value.take()?)),
            Self::TupleStruct(value) => Self::TupleStruct(Shared::new(value.take()?)),
            Self::Struct(value) => Self::Struct(Shared::new(value.take()?)),
            Self::UnitVariant(value) => Self::UnitVariant(Shared::new(value.take()?)),
            Self::TupleVariant(value) => Self::TupleVariant(Shared::new(value.take()?)),
            Self::StructVariant(value) => Self::StructVariant(Shared::new(value.take()?)),
            Self::Function(value) => Self::Function(Shared::new(value.take()?)),
            Self::Format(value) => Self::Format(value),
            Self::Any(value) => Self::Any(Shared::new(value.take()?)),
        })
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

    /// Try to coerce value into a boolean.
    #[inline]
    pub fn as_bool(&self) -> Result<bool, VmError> {
        match self {
            Self::Bool(b) => Ok(*b),
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
    pub fn into_vec(self) -> Result<Shared<Vec>, VmError> {
        match self {
            Self::Vec(vec) => Ok(vec),
            actual => Err(VmError::expected::<Vec>(actual.type_info()?)),
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

    /// Try to coerce value into a format spec.
    #[inline]
    pub fn into_format(self) -> Result<Box<Format>, VmError> {
        match self {
            Value::Format(format) => Ok(format),
            actual => Err(VmError::expected::<Format>(actual.type_info()?)),
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
    pub fn into_any_ptr<T>(self) -> Result<(*const T, RawRef), VmError>
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
    pub fn into_any_mut<T>(self) -> Result<(*mut T, RawMut), VmError>
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
            Self::Format(..) => Type::from(crate::FORMAT_TYPE),
            Self::Type(hash) => Type::from(*hash),
            Self::UnitStruct(empty) => Type::from(empty.borrow_ref()?.rtti.hash),
            Self::TupleStruct(tuple) => Type::from(tuple.borrow_ref()?.rtti.hash),
            Self::Struct(object) => Type::from(object.borrow_ref()?.rtti.hash),
            Self::UnitVariant(empty) => Type::from(empty.borrow_ref()?.rtti.enum_hash),
            Self::TupleVariant(tuple) => Type::from(tuple.borrow_ref()?.rtti.enum_hash),
            Self::StructVariant(object) => Type::from(object.borrow_ref()?.rtti.enum_hash),
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
            Self::Format(..) => TypeInfo::StaticType(crate::FORMAT_TYPE),
            Self::Type(hash) => TypeInfo::Hash(*hash),
            Self::UnitStruct(empty) => empty.borrow_ref()?.type_info(),
            Self::TupleStruct(tuple) => tuple.borrow_ref()?.type_info(),
            Self::Struct(object) => object.borrow_ref()?.type_info(),
            Self::UnitVariant(empty) => empty.borrow_ref()?.type_info(),
            Self::TupleVariant(tuple) => tuple.borrow_ref()?.type_info(),
            Self::StructVariant(object) => object.borrow_ref()?.type_info(),
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
            (Self::Bool(a), Self::Bool(b)) => a == b,
            (Self::Byte(a), Self::Byte(b)) => a == b,
            (Self::Char(a), Self::Char(b)) => a == b,
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
            (Self::Tuple(a), Self::Tuple(b)) => {
                let a = a.borrow_ref()?;
                let b = b.borrow_ref()?;
                Tuple::value_ptr_eq(&*a, &*b)?
            }
            (Self::Object(a), Self::Object(b)) => {
                let a = a.borrow_ref()?;
                let b = b.borrow_ref()?;
                Object::value_ptr_eq(&*a, &*b)?
            }
            (Self::UnitStruct(a), Self::UnitStruct(b)) => {
                a.borrow_ref()?.rtti.hash == b.borrow_ref()?.rtti.hash
            }
            (Self::TupleStruct(a), Self::TupleStruct(b)) => {
                let a = a.borrow_ref()?;
                let b = b.borrow_ref()?;

                a.rtti.hash == b.rtti.hash && Tuple::value_ptr_eq(&a.data, &b.data)?
            }
            (Self::Struct(a), Self::Struct(b)) => {
                let a = a.borrow_ref()?;
                let b = b.borrow_ref()?;

                a.rtti.hash == b.rtti.hash && Object::value_ptr_eq(&a.data, &b.data)?
            }
            (Self::UnitVariant(a), Self::UnitVariant(b)) => {
                a.borrow_ref()?.rtti.hash == b.borrow_ref()?.rtti.hash
            }
            (Self::TupleVariant(a), Self::TupleVariant(b)) => {
                let a = a.borrow_ref()?;
                let b = b.borrow_ref()?;

                a.rtti.hash == b.rtti.hash && Tuple::value_ptr_eq(&a.data, &b.data)?
            }
            (Self::StructVariant(a), Self::StructVariant(b)) => {
                let a = a.borrow_ref()?;
                let b = b.borrow_ref()?;

                a.rtti.hash == b.rtti.hash && Object::value_ptr_eq(&a.data, &b.data)?
            }
            (Self::String(a), Self::String(b)) => *a.borrow_ref()? == *b.borrow_ref()?,
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
            (Self::Option(a), Self::Option(b)) => match (&*a.borrow_ref()?, &*b.borrow_ref()?) {
                (Some(a), Some(b)) => Self::value_ptr_eq(a, b)?,
                (None, None) => true,
                _ => false,
            },
            (Self::Result(a), Self::Result(b)) => match (&*a.borrow_ref()?, &*b.borrow_ref()?) {
                (Ok(a), Ok(b)) => Self::value_ptr_eq(a, b)?,
                (Err(a), Err(b)) => Self::value_ptr_eq(a, b)?,
                _ => false,
            },
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
            Value::UnitStruct(value) => {
                write!(f, "{:?}", value)?;
            }
            Value::TupleStruct(value) => {
                write!(f, "{:?}", value)?;
            }
            Value::Struct(value) => {
                write!(f, "{:?}", value)?;
            }
            Value::UnitVariant(value) => {
                write!(f, "{:?}", value)?;
            }
            Value::TupleVariant(value) => {
                write!(f, "{:?}", value)?;
            }
            Value::StructVariant(value) => {
                write!(f, "{:?}", value)?;
            }
            Value::Function(value) => {
                write!(f, "{:?}", value)?;
            }
            Value::Format(value) => {
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
        Ok(Value::from(()))
    }
}

macro_rules! impl_from {
    ($($variant:ident => $ty:ty),* $(,)*) => {
        $(
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
        )*
    };
}

macro_rules! impl_from_wrapper {
    ($($variant:ident => $wrapper:ident<$ty:ty>),* $(,)?) => {
        impl_from!($($variant => $wrapper<$ty>),*);

        $(
            impl From<$ty> for Value {
                fn from(value: $ty) -> Self {
                    Self::$variant($wrapper::new(value))
                }
            }

            impl $crate::ToValue for $ty {
                fn to_value(self) -> Result<Value, VmError> {
                    Ok(Value::from(self))
                }
            }
        )*
    };
}

impl_from! {
    Byte => u8,
    Bool => bool,
    Char => char,
    Integer => i64,
    Float => f64,
    Option => Shared<Option<Value>>,
    Result => Shared<Result<Value, Value>>,
}

impl_from_wrapper! {
    StaticString => Arc<StaticString>,
    Format => Box<Format>,
    Bytes => Shared<Bytes>,
    String => Shared<String>,
    Vec => Shared<Vec>,
    Tuple => Shared<Tuple>,
    Object => Shared<Object>,
    Future => Shared<Future>,
    Stream => Shared<Stream>,
    Generator => Shared<Generator>,
    GeneratorState => Shared<GeneratorState>,
    UnitStruct => Shared<UnitStruct>,
    TupleStruct => Shared<TupleStruct>,
    Struct => Shared<Struct>,
    UnitVariant => Shared<UnitVariant>,
    TupleVariant => Shared<TupleVariant>,
    StructVariant => Shared<StructVariant>,
    Function => Shared<Function>,
    Any => Shared<AnyObj>,
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
