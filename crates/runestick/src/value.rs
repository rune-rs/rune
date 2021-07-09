use crate::access::AccessKind;
use crate::protocol_caller::{EnvProtocolCaller, ProtocolCaller};
use crate::{
    Any, AnyObj, Bytes, ConstValue, Format, Function, Future, Generator, GeneratorState, Hash,
    Item, Iterator, Mut, Object, Protocol, Range, RawMut, RawRef, Ref, Shared, StaticString,
    Stream, Tuple, TypeInfo, Variant, Vec, Vm, VmError, VmErrorKind,
};
use serde::{de, ser, Deserialize, Serialize};
use std::cmp;
use std::fmt;
use std::fmt::Write;
use std::hash;
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
        Q: std::hash::Hash + std::cmp::Eq + std::cmp::Ord,
    {
        self.data.get(k)
    }

    /// Get the given mutable value by key in the object.
    pub fn get_mut<Q: ?Sized>(&mut self, k: &Q) -> Option<&mut Value>
    where
        String: std::borrow::Borrow<Q>,
        Q: std::hash::Hash + std::cmp::Eq + std::cmp::Ord,
    {
        self.data.get_mut(k)
    }
}

impl fmt::Debug for Struct {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.data.debug_struct(&self.rtti.item))
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

impl cmp::PartialEq for VariantRtti {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
    }
}

impl cmp::Eq for VariantRtti {}

impl hash::Hash for VariantRtti {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.hash.hash(state)
    }
}

impl cmp::PartialOrd for VariantRtti {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.hash.partial_cmp(&other.hash)
    }
}

impl cmp::Ord for VariantRtti {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.hash.cmp(&other.hash)
    }
}

/// Runtime information on variant.
#[derive(Debug, Serialize, Deserialize)]
pub struct Rtti {
    /// The type hash of the type.
    pub hash: Hash,
    /// The item of the type.
    pub item: Item,
}

impl cmp::PartialEq for Rtti {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
    }
}

impl cmp::Eq for Rtti {}

impl hash::Hash for Rtti {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.hash.hash(state)
    }
}

impl cmp::PartialOrd for Rtti {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.hash.partial_cmp(&other.hash)
    }
}

impl cmp::Ord for Rtti {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.hash.cmp(&other.hash)
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
    Vec(Shared<Vec>),
    /// A tuple.
    Tuple(Shared<Tuple>),
    /// An object.
    Object(Shared<Object>),
    /// A range.
    Range(Shared<Range>),
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
    /// The variant of an enum.
    Variant(Shared<Variant>),
    /// A stored function pointer.
    Function(Shared<Function>),
    /// A value being formatted.
    Format(Box<Format>),
    /// An iterator.
    Iterator(Shared<Iterator>),
    /// An opaque value that can be downcasted.
    Any(Shared<AnyObj>),
}

impl Value {
    /// Format the value using the [Protocol::STRING_DISPLAY] protocol.
    ///
    /// Also requires a work buffer `buf` which will be used in case the value
    /// provided required formatting.
    ///
    /// Note that this function will always failed if called outside of a
    /// virtual machine.
    pub fn string_display(&self, s: &mut String, buf: &mut String) -> Result<fmt::Result, VmError> {
        self.string_display_with(s, buf, EnvProtocolCaller)
    }

    /// Internal impl of string_display with a customizable caller.
    pub(crate) fn string_display_with(
        &self,
        s: &mut String,
        buf: &mut String,
        caller: impl ProtocolCaller,
    ) -> Result<fmt::Result, VmError> {
        use crate::FromValue as _;

        match self {
            Value::Format(format) => {
                format.spec.format(&format.value, s, buf, caller)?;
            }
            Value::Char(c) => {
                s.push(*c);
            }
            Value::String(string) => {
                s.push_str(&string.borrow_ref()?);
            }
            Value::StaticString(string) => {
                s.push_str(string.as_ref());
            }
            Value::Integer(integer) => {
                let mut buffer = itoa::Buffer::new();
                s.push_str(buffer.format(*integer));
            }
            Value::Float(float) => {
                let mut buffer = ryu::Buffer::new();
                s.push_str(buffer.format(*float));
            }
            Value::Bool(bool) => {
                return Ok(write!(s, "{}", bool));
            }
            Value::Byte(byte) => {
                return Ok(write!(s, "{:#04X}", byte));
            }
            value => {
                let b = Shared::new(std::mem::take(s));

                let result = caller.call_protocol_fn(
                    Protocol::STRING_DISPLAY,
                    value.clone(),
                    (Value::from(b.clone()),),
                )?;

                let result = fmt::Result::from_value(result)?;
                drop(std::mem::replace(s, b.take()?));
                return Ok(result);
            }
        }

        Ok(Ok(()))
    }

    /// Debug format the value using the [Protocol::STRING_DEBUG] protocol.
    ///
    /// Note that this function will always failed if called outside of a
    /// virtual machine.
    pub fn string_debug(&self, s: &mut String) -> Result<fmt::Result, VmError> {
        self.string_debug_with(s, EnvProtocolCaller)
    }

    /// Internal impl of string_debug with a customizable caller.
    pub(crate) fn string_debug_with(
        &self,
        s: &mut String,
        caller: impl ProtocolCaller,
    ) -> Result<fmt::Result, VmError> {
        use crate::FromValue as _;
        use std::fmt::Write as _;

        let result = match self {
            Value::Unit => {
                write!(s, "()")
            }
            Value::Bool(value) => {
                write!(s, "{:?}", value)
            }
            Value::Byte(value) => {
                write!(s, "{:?}", value)
            }
            Value::Char(value) => {
                write!(s, "{:?}", value)
            }
            Value::Integer(value) => {
                write!(s, "{:?}", value)
            }
            Value::Float(value) => {
                write!(s, "{:?}", value)
            }
            Value::Type(value) => {
                write!(s, "Type({})", value)
            }
            Value::StaticString(value) => {
                write!(s, "{:?}", value)
            }
            Value::String(value) => {
                write!(s, "{:?}", value)
            }
            Value::Bytes(value) => {
                write!(s, "{:?}", value)
            }
            Value::Vec(value) => {
                write!(s, "{:?}", value)
            }
            Value::Tuple(value) => {
                write!(s, "{:?}", value)
            }
            Value::Object(value) => {
                write!(s, "{:?}", value)
            }
            Value::Range(value) => {
                write!(s, "{:?}", value)
            }
            Value::Future(value) => {
                write!(s, "{:?}", value)
            }
            Value::Stream(value) => {
                write!(s, "{:?}", value)
            }
            Value::Generator(value) => {
                write!(s, "{:?}", value)
            }
            Value::GeneratorState(value) => {
                write!(s, "{:?}", value)
            }
            Value::Option(value) => {
                write!(s, "{:?}", value)
            }
            Value::Result(value) => {
                write!(s, "{:?}", value)
            }
            Value::UnitStruct(value) => {
                write!(s, "{:?}", value)
            }
            Value::TupleStruct(value) => {
                write!(s, "{:?}", value)
            }
            Value::Struct(value) => {
                write!(s, "{:?}", value)
            }
            Value::Variant(value) => {
                write!(s, "{:?}", value)
            }
            Value::Function(value) => {
                write!(s, "{:?}", value)
            }
            Value::Format(value) => {
                write!(s, "{:?}", value)
            }
            Value::Iterator(value) => {
                write!(s, "{:?}", value)
            }
            value => {
                let b = Shared::new(std::mem::take(s));

                let result = caller.call_protocol_fn(
                    Protocol::STRING_DEBUG,
                    value.clone(),
                    (Value::from(b.clone()),),
                )?;

                let result = fmt::Result::from_value(result)?;
                drop(std::mem::replace(s, b.take()?));
                return Ok(result);
            }
        };

        Ok(result)
    }

    /// Convert value into an iterator using the [Protocol::INTO_ITER] protocol.
    ///
    /// Note that this function will always failed if called outside of a
    /// virtual machine.
    #[allow(clippy::should_implement_trait)]
    pub fn into_iter(self) -> Result<Iterator, VmError> {
        use crate::FromValue as _;

        let target = match self {
            Value::Iterator(iterator) => return Ok(iterator.take()?),
            Value::Vec(vec) => return Ok(vec.borrow_ref()?.into_iterator()),
            Value::Object(object) => return Ok(object.borrow_ref()?.into_iterator()),
            target => target,
        };

        let value = EnvProtocolCaller.call_protocol_fn(Protocol::INTO_ITER, target, ())?;
        Iterator::from_value(value)
    }

    /// Coerce into future, or convert into a future using the
    /// [Protocol::INTO_FUTURE] protocol.
    ///
    /// Note that this function will always failed if called outside of a
    /// virtual machine.
    pub fn into_future(self) -> Result<Future, VmError> {
        use crate::FromValue as _;

        let target = match self {
            Value::Future(fut) => return Ok(fut.take()?),
            target => target,
        };

        let value = EnvProtocolCaller.call_protocol_fn(Protocol::INTO_FUTURE, target, ())?;
        Future::from_value(value)
    }

    /// Coerce into a shared future, or convert into a future using the
    /// [Protocol::INTO_FUTURE] protocol.
    ///
    /// Note that this function will always failed if called outside of a
    /// virtual machine.
    #[inline]
    pub fn into_shared_future(self) -> Result<Shared<Future>, VmError> {
        use crate::FromValue as _;

        let target = match self {
            Value::Future(future) => return Ok(future),
            target => target,
        };

        let value = EnvProtocolCaller.call_protocol_fn(Protocol::INTO_FUTURE, target, ())?;
        Ok(Shared::new(Future::from_value(value)?))
    }

    /// Retrieves a human readable type name for the current value.
    ///
    /// Note that this function will always failed if called outside of a
    /// virtual machine.
    pub fn into_type_name(self) -> Result<String, VmError> {
        let hash = Hash::instance_function(self.type_hash()?, Protocol::INTO_TYPE_NAME);

        crate::env::with(|context, unit| {
            if let Some(name) = context.constant(hash) {
                match name {
                    ConstValue::String(s) => return Ok(s.clone()),
                    ConstValue::StaticString(s) => return Ok((*s).to_string()),
                    _ => return Err(VmError::expected::<String>(name.type_info())),
                }
            }

            if let Some(name) = unit.constant(hash) {
                match name {
                    ConstValue::String(s) => return Ok(s.clone()),
                    ConstValue::StaticString(s) => return Ok((*s).to_string()),
                    _ => return Err(VmError::expected::<String>(name.type_info())),
                }
            }

            self.type_info().map(|v| format!("{}", v))
        })
    }

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
    pub fn unit_variant(rtti: Arc<VariantRtti>) -> Self {
        Self::Variant(Shared::new(Variant::unit(rtti)))
    }

    /// Construct a tuple variant.
    pub fn tuple_variant(rtti: Arc<VariantRtti>, vec: vec::Vec<Value>) -> Self {
        Self::Variant(Shared::new(Variant::tuple(rtti, Tuple::from(vec))))
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
            Self::Range(value) => Self::Range(Shared::new(value.take()?)),
            Self::Future(value) => Self::Future(Shared::new(value.take()?)),
            Self::Stream(value) => Self::Stream(Shared::new(value.take()?)),
            Self::Generator(value) => Self::Generator(Shared::new(value.take()?)),
            Self::GeneratorState(value) => Self::GeneratorState(Shared::new(value.take()?)),
            Self::Option(value) => Self::Option(Shared::new(value.take()?)),
            Self::Result(value) => Self::Result(Shared::new(value.take()?)),
            Self::UnitStruct(value) => Self::UnitStruct(Shared::new(value.take()?)),
            Self::TupleStruct(value) => Self::TupleStruct(Shared::new(value.take()?)),
            Self::Struct(value) => Self::Struct(Shared::new(value.take()?)),
            Self::Variant(value) => Self::Variant(Shared::new(value.take()?)),
            Self::Function(value) => Self::Function(Shared::new(value.take()?)),
            Self::Format(value) => Self::Format(value),
            Self::Iterator(value) => Self::Iterator(value),
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

    /// Try to coerce value into a range.
    #[inline]
    pub fn into_range(self) -> Result<Shared<Range>, VmError> {
        match self {
            Self::Range(object) => Ok(object),
            actual => Err(VmError::expected::<Range>(actual.type_info()?)),
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

    /// Try to coerce value into an iterator.
    #[inline]
    pub fn into_iterator(self) -> Result<Shared<Iterator>, VmError> {
        match self {
            Value::Iterator(format) => Ok(format),
            actual => Err(VmError::expected::<Iterator>(actual.type_info()?)),
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

    /// Get the type hash for the current value.
    pub fn type_hash(&self) -> Result<Hash, VmError> {
        Ok(match self {
            Self::Unit => crate::UNIT_TYPE.hash,
            Self::Bool(..) => crate::BOOL_TYPE.hash,
            Self::Byte(..) => crate::BYTE_TYPE.hash,
            Self::Char(..) => crate::CHAR_TYPE.hash,
            Self::Integer(..) => crate::INTEGER_TYPE.hash,
            Self::Float(..) => crate::FLOAT_TYPE.hash,
            Self::StaticString(..) => crate::STRING_TYPE.hash,
            Self::String(..) => crate::STRING_TYPE.hash,
            Self::Bytes(..) => crate::BYTES_TYPE.hash,
            Self::Vec(..) => crate::VEC_TYPE.hash,
            Self::Tuple(..) => crate::TUPLE_TYPE.hash,
            Self::Object(..) => crate::OBJECT_TYPE.hash,
            Self::Range(..) => crate::RANGE_TYPE.hash,
            Self::Future(..) => crate::FUTURE_TYPE.hash,
            Self::Stream(..) => crate::STREAM_TYPE.hash,
            Self::Generator(..) => crate::GENERATOR_TYPE.hash,
            Self::GeneratorState(..) => crate::GENERATOR_STATE_TYPE.hash,
            Self::Result(..) => crate::RESULT_TYPE.hash,
            Self::Option(..) => crate::OPTION_TYPE.hash,
            Self::Function(func) => func.borrow_ref()?.type_hash(),
            Self::Format(..) => crate::FORMAT_TYPE.hash,
            Self::Iterator(..) => crate::ITERATOR_TYPE.hash,
            Self::Type(hash) => *hash,
            Self::UnitStruct(empty) => empty.borrow_ref()?.rtti.hash,
            Self::TupleStruct(tuple) => tuple.borrow_ref()?.rtti.hash,
            Self::Struct(object) => object.borrow_ref()?.rtti.hash,
            Self::Variant(variant) => variant.borrow_ref()?.rtti().enum_hash,
            Self::Any(any) => any.borrow_ref()?.type_hash(),
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
            Self::Range(..) => TypeInfo::StaticType(crate::RANGE_TYPE),
            Self::Future(..) => TypeInfo::StaticType(crate::FUTURE_TYPE),
            Self::Stream(..) => TypeInfo::StaticType(crate::STREAM_TYPE),
            Self::Generator(..) => TypeInfo::StaticType(crate::GENERATOR_TYPE),
            Self::GeneratorState(..) => TypeInfo::StaticType(crate::GENERATOR_STATE_TYPE),
            Self::Option(..) => TypeInfo::StaticType(crate::OPTION_TYPE),
            Self::Result(..) => TypeInfo::StaticType(crate::RESULT_TYPE),
            Self::Function(..) => TypeInfo::StaticType(crate::FUNCTION_TYPE),
            Self::Format(..) => TypeInfo::StaticType(crate::FORMAT_TYPE),
            Self::Iterator(..) => TypeInfo::StaticType(crate::ITERATOR_TYPE),
            Self::Type(..) => TypeInfo::StaticType(crate::TYPE),
            Self::UnitStruct(empty) => empty.borrow_ref()?.type_info(),
            Self::TupleStruct(tuple) => tuple.borrow_ref()?.type_info(),
            Self::Struct(object) => object.borrow_ref()?.type_info(),
            Self::Variant(empty) => empty.borrow_ref()?.type_info(),
            Self::Any(any) => TypeInfo::Any(any.borrow_ref()?.type_name()),
        })
    }

    /// Optimized function to test if two value pointers are deeply equal to
    /// each other.
    ///
    /// This is the basis for the eq operation (`==`).
    pub(crate) fn value_ptr_eq(vm: &mut Vm, a: &Value, b: &Value) -> Result<bool, VmError> {
        match (a, b) {
            (Self::Unit, Self::Unit) => return Ok(true),
            (Self::Bool(a), Self::Bool(b)) => return Ok(a == b),
            (Self::Byte(a), Self::Byte(b)) => return Ok(a == b),
            (Self::Char(a), Self::Char(b)) => return Ok(a == b),
            (Self::Integer(a), Self::Integer(b)) => return Ok(a == b),
            (Self::Float(a), Self::Float(b)) => return Ok(a == b),
            (Self::Vec(a), Self::Vec(b)) => {
                let a = a.borrow_ref()?;
                let b = b.borrow_ref()?;
                return Vec::value_ptr_eq(vm, &*a, &*b);
            }
            (Self::Tuple(a), Self::Tuple(b)) => {
                let a = a.borrow_ref()?;
                let b = b.borrow_ref()?;
                return Tuple::value_ptr_eq(vm, &*a, &*b);
            }
            (Self::Object(a), Self::Object(b)) => {
                let a = a.borrow_ref()?;
                let b = b.borrow_ref()?;
                return Object::value_ptr_eq(vm, &*a, &*b);
            }
            (Self::Range(a), Self::Range(b)) => {
                let a = a.borrow_ref()?;
                let b = b.borrow_ref()?;
                return Range::value_ptr_eq(vm, &*a, &*b);
            }
            (Self::UnitStruct(a), Self::UnitStruct(b)) => {
                if a.borrow_ref()?.rtti.hash == b.borrow_ref()?.rtti.hash {
                    // NB: don't get any future ideas, this must fall through to
                    // the VmError below since it's otherwise a comparison
                    // between two incompatible types.
                    //
                    // Other than that, all units are equal.
                    return Ok(true);
                }
            }
            (Self::TupleStruct(a), Self::TupleStruct(b)) => {
                let a = a.borrow_ref()?;
                let b = b.borrow_ref()?;

                if a.rtti.hash == b.rtti.hash {
                    return Tuple::value_ptr_eq(vm, &a.data, &b.data);
                }
            }
            (Self::Struct(a), Self::Struct(b)) => {
                let a = a.borrow_ref()?;
                let b = b.borrow_ref()?;

                if a.rtti.hash == b.rtti.hash {
                    return Object::value_ptr_eq(vm, &a.data, &b.data);
                }
            }
            (Self::Variant(a), Self::Variant(b)) => {
                let a = a.borrow_ref()?;
                let b = b.borrow_ref()?;

                if a.rtti().enum_hash == b.rtti().enum_hash {
                    return Variant::value_ptr_eq(vm, &*a, &*b);
                }
            }
            (Self::String(a), Self::String(b)) => {
                return Ok(*a.borrow_ref()? == *b.borrow_ref()?);
            }
            (Self::StaticString(a), Self::String(b)) => {
                let b = b.borrow_ref()?;
                return Ok(***a == *b);
            }
            (Self::String(a), Self::StaticString(b)) => {
                let a = a.borrow_ref()?;
                return Ok(*a == ***b);
            }
            // fast string comparison: exact string slot.
            (Self::StaticString(a), Self::StaticString(b)) => {
                return Ok(***a == ***b);
            }
            (Self::Option(a), Self::Option(b)) => match (&*a.borrow_ref()?, &*b.borrow_ref()?) {
                (Some(a), Some(b)) => return Self::value_ptr_eq(vm, a, b),
                (None, None) => return Ok(true),
                _ => return Ok(false),
            },
            (Self::Result(a), Self::Result(b)) => match (&*a.borrow_ref()?, &*b.borrow_ref()?) {
                (Ok(a), Ok(b)) => return Self::value_ptr_eq(vm, a, b),
                (Err(a), Err(b)) => return Self::value_ptr_eq(vm, a, b),
                _ => return Ok(false),
            },
            (a, b) => {
                if vm.call_instance_fn(a.clone(), Protocol::EQ, (b.clone(),))? {
                    use crate::FromValue as _;
                    return bool::from_value(vm.stack.pop()?);
                }
            }
        }

        Err(VmError::from(VmErrorKind::UnsupportedBinaryOperation {
            op: "==",
            lhs: a.type_info()?,
            rhs: b.type_info()?,
        }))
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
            Value::Range(value) => {
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
            Value::Variant(value) => {
                write!(f, "{:?}", value)?;
            }
            Value::Function(value) => {
                write!(f, "{:?}", value)?;
            }
            Value::Format(value) => {
                write!(f, "{:?}", value)?;
            }
            Value::Iterator(value) => {
                write!(f, "{:?}", value)?;
            }
            value => {
                let mut s = String::new();
                let result = match value.string_debug(&mut s) {
                    Ok(s) => s,
                    Err(_) => {
                        // this isn't very nice, but if protocol string fails
                        // this used to crash out immediately... in this way,
                        // one can at least get soms semblance of info for all
                        // types. And if the type doesn't have type info
                        // something else has gone terribly wrong.
                        s = format!("{:?}", value.type_info().unwrap());
                        Ok(())
                    }
                };
                result?;

                write!(f, "{}", s)?;
            }
        }

        Ok(())
    }
}

impl Default for Value {
    fn default() -> Self {
        Self::Unit
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
    Iterator => Shared<Iterator>,
    Bytes => Shared<Bytes>,
    String => Shared<String>,
    Vec => Shared<Vec>,
    Tuple => Shared<Tuple>,
    Object => Shared<Object>,
    Range => Shared<Range>,
    Future => Shared<Future>,
    Stream => Shared<Stream>,
    Generator => Shared<Generator>,
    GeneratorState => Shared<GeneratorState>,
    UnitStruct => Shared<UnitStruct>,
    TupleStruct => Shared<TupleStruct>,
    Struct => Shared<Struct>,
    Variant => Shared<Variant>,
    Function => Shared<Function>,
    Any => Shared<AnyObj>,
}

/// Deserialize implementation for value pointers.
impl<'de> de::Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_any(VmVisitor)
    }
}

/// Serialize implementation for value pointers.
impl ser::Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        use serde::ser::SerializeMap as _;
        use serde::ser::SerializeSeq as _;

        match self {
            Value::Unit => serializer.serialize_unit(),
            Value::Bool(b) => serializer.serialize_bool(*b),
            Value::Char(c) => serializer.serialize_char(*c),
            Value::Byte(c) => serializer.serialize_u8(*c),
            Value::Integer(integer) => serializer.serialize_i64(*integer),
            Value::Float(float) => serializer.serialize_f64(*float),
            Value::StaticString(string) => serializer.serialize_str(string.as_ref()),
            Value::String(string) => {
                let string = string.borrow_ref().map_err(ser::Error::custom)?;
                serializer.serialize_str(&*string)
            }
            Value::Bytes(bytes) => {
                let bytes = bytes.borrow_ref().map_err(ser::Error::custom)?;
                serializer.serialize_bytes(&*bytes)
            }
            Value::Vec(vec) => {
                let vec = vec.borrow_ref().map_err(ser::Error::custom)?;
                let mut serializer = serializer.serialize_seq(Some(vec.len()))?;

                for value in &*vec {
                    serializer.serialize_element(value)?;
                }

                serializer.end()
            }
            Value::Tuple(tuple) => {
                let tuple = tuple.borrow_ref().map_err(ser::Error::custom)?;
                let mut serializer = serializer.serialize_seq(Some(tuple.len()))?;

                for value in tuple.iter() {
                    serializer.serialize_element(value)?;
                }

                serializer.end()
            }
            Value::Object(object) => {
                let object = object.borrow_ref().map_err(ser::Error::custom)?;
                let mut serializer = serializer.serialize_map(Some(object.len()))?;

                for (key, value) in &*object {
                    serializer.serialize_entry(key, value)?;
                }

                serializer.end()
            }
            Value::Option(option) => {
                let option = option.borrow_ref().map_err(ser::Error::custom)?;
                <Option<Value>>::serialize(&*option, serializer)
            }
            Value::UnitStruct(..) => serializer.serialize_unit(),
            Value::TupleStruct(..) => Err(ser::Error::custom("cannot serialize tuple structs")),
            Value::Struct(..) => Err(ser::Error::custom("cannot serialize objects structs")),
            Value::Variant(..) => Err(ser::Error::custom("cannot serialize variants")),
            Value::Result(..) => Err(ser::Error::custom("cannot serialize results")),
            Value::Type(..) => Err(ser::Error::custom("cannot serialize types")),
            Value::Future(..) => Err(ser::Error::custom("cannot serialize futures")),
            Value::Stream(..) => Err(ser::Error::custom("cannot serialize streams")),
            Value::Generator(..) => Err(ser::Error::custom("cannot serialize generators")),
            Value::GeneratorState(..) => {
                Err(ser::Error::custom("cannot serialize generator states"))
            }
            Value::Function(..) => Err(ser::Error::custom("cannot serialize function pointers")),
            Value::Format(..) => Err(ser::Error::custom("cannot serialize format specifications")),
            Value::Iterator(..) => Err(ser::Error::custom("cannot serialize iterators")),
            Value::Range(..) => Err(ser::Error::custom("cannot serialize ranges")),
            Value::Any(..) => Err(ser::Error::custom("cannot serialize external objects")),
        }
    }
}

struct VmVisitor;

impl<'de> de::Visitor<'de> for VmVisitor {
    type Value = Value;

    fn expecting(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.write_str("any valid value")
    }

    #[inline]
    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::String(Shared::new(value.to_owned())))
    }

    #[inline]
    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::String(Shared::new(value)))
    }

    #[inline]
    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::Bytes(Shared::new(Bytes::from_vec(v.to_vec()))))
    }

    #[inline]
    fn visit_byte_buf<E>(self, v: vec::Vec<u8>) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::Bytes(Shared::new(Bytes::from_vec(v))))
    }

    #[inline]
    fn visit_i8<E>(self, v: i8) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::Integer(v as i64))
    }

    #[inline]
    fn visit_i16<E>(self, v: i16) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::Integer(v as i64))
    }

    #[inline]
    fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::Integer(v as i64))
    }

    #[inline]
    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::Integer(v))
    }

    #[inline]
    fn visit_i128<E>(self, v: i128) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::Integer(v as i64))
    }

    #[inline]
    fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::Integer(v as i64))
    }

    #[inline]
    fn visit_u16<E>(self, v: u16) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::Integer(v as i64))
    }

    #[inline]
    fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::Integer(v as i64))
    }

    #[inline]
    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::Integer(v as i64))
    }

    #[inline]
    fn visit_u128<E>(self, v: u128) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::Integer(v as i64))
    }

    #[inline]
    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::Bool(v))
    }

    #[inline]
    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::Unit)
    }

    #[inline]
    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::Unit)
    }

    #[inline]
    fn visit_seq<V>(self, mut visitor: V) -> Result<Self::Value, V::Error>
    where
        V: de::SeqAccess<'de>,
    {
        let mut vec = if let Some(hint) = visitor.size_hint() {
            vec::Vec::with_capacity(hint)
        } else {
            vec::Vec::new()
        };

        while let Some(elem) = visitor.next_element()? {
            vec.push(elem);
        }

        Ok(Value::Vec(Shared::new(Vec::from(vec))))
    }

    #[inline]
    fn visit_map<V>(self, mut visitor: V) -> Result<Self::Value, V::Error>
    where
        V: de::MapAccess<'de>,
    {
        let mut object = Object::new();

        while let Some((key, value)) = visitor.next_entry()? {
            object.insert(key, value);
        }

        Ok(Value::Object(Shared::new(object)))
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
