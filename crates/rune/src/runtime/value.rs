use crate::compile::ItemBuf;
use crate::runtime::vm::CallResult;
use crate::runtime::{
    AccessKind, AnyObj, Bytes, ConstValue, EnvProtocolCaller, Format, FromValue, FullTypeOf,
    Function, Future, Generator, GeneratorState, Iterator, MaybeTypeOf, Mut, Object, Protocol,
    ProtocolCaller, Range, RawMut, RawRef, Ref, Shared, StaticString, Stream, ToValue, Tuple,
    TypeInfo, Variant, Vec, Vm, VmErrorKind, VmResult,
};
use crate::{Any, Hash};
use serde::{de, ser, Deserialize, Serialize};
use std::cmp;
use std::fmt;
use std::fmt::Write;
use std::hash;
use std::sync::Arc;
use std::vec;

use VmResult::Err;
use VmResult::Ok;

// Small helper function to build errors.
fn err<T, E>(error: E) -> VmResult<T>
where
    VmErrorKind: From<E>,
{
    VmResult::err(error)
}

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
#[non_exhaustive]
pub struct VariantRtti {
    /// The type hash of the enum.
    pub enum_hash: Hash,
    /// The type variant hash.
    pub hash: Hash,
    /// The name of the variant.
    pub item: ItemBuf,
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
#[non_exhaustive]
pub struct Rtti {
    /// The type hash of the type.
    pub hash: Hash,
    /// The item of the type.
    pub item: ItemBuf,
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
    Stream(Shared<Stream<Vm>>),
    /// A stored generator.
    Generator(Shared<Generator<Vm>>),
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
    /// Requires a work buffer `buf` which will be used in case the value
    /// provided requires out-of-line formatting. This must be cleared between
    /// calls and can be re-used.
    ///
    /// You must use [Vm::with] to specify which virtual machine this function
    /// is called inside.
    ///
    /// # Panics
    ///
    /// This function will panic if called outside of a virtual machine.
    pub fn string_display(&self, s: &mut String, buf: &mut String) -> VmResult<fmt::Result> {
        self.string_display_with(s, buf, EnvProtocolCaller)
    }

    /// Internal impl of string_display with a customizable caller.
    pub(crate) fn string_display_with(
        &self,
        s: &mut String,
        buf: &mut String,
        caller: impl ProtocolCaller,
    ) -> VmResult<fmt::Result> {
        match self {
            Value::Format(format) => {
                vm_try!(format.spec.format(&format.value, s, buf, caller));
            }
            Value::Char(c) => {
                s.push(*c);
            }
            Value::String(string) => {
                s.push_str(&vm_try!(string.borrow_ref()));
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

                let result = vm_try!(caller.call_protocol_fn(
                    Protocol::STRING_DISPLAY,
                    value.clone(),
                    (Value::from(b.clone()),),
                ));

                let result = vm_try!(fmt::Result::from_value(result));
                drop(std::mem::replace(s, vm_try!(b.take())));
                return Ok(result);
            }
        }

        Ok(fmt::Result::Ok(()))
    }

    /// Debug format the value using the [Protocol::STRING_DEBUG] protocol.
    ///
    /// You must use [Vm::with] to specify which virtual machine this function
    /// is called inside.
    ///
    /// # Panics
    ///
    /// This function will panic if called outside of a virtual machine.
    pub fn string_debug(&self, s: &mut String) -> VmResult<fmt::Result> {
        self.string_debug_with(s, EnvProtocolCaller)
    }

    /// Internal impl of string_debug with a customizable caller.
    pub(crate) fn string_debug_with(
        &self,
        s: &mut String,
        caller: impl ProtocolCaller,
    ) -> VmResult<fmt::Result> {
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

                let result = vm_try!(caller.call_protocol_fn(
                    Protocol::STRING_DEBUG,
                    value.clone(),
                    (Value::from(b.clone()),),
                ));

                let result = vm_try!(fmt::Result::from_value(result));
                drop(std::mem::replace(s, vm_try!(b.take())));
                return Ok(result);
            }
        };

        Ok(result)
    }

    /// Convert value into an iterator using the [Protocol::INTO_ITER] protocol.
    ///
    /// You must use [Vm::with] to specify which virtual machine this function
    /// is called inside.
    ///
    /// # Errors
    ///
    /// This function errors in case the provided type cannot be converted into
    /// an iterator without the use of a [`Vm`] and one is not provided through
    /// the environment.
    pub fn into_iter(self) -> VmResult<Iterator> {
        let target = match self {
            Value::Iterator(iterator) => return Ok(vm_try!(iterator.take())),
            Value::Vec(vec) => return Ok(vm_try!(vec.borrow_ref()).into_iterator()),
            Value::Object(object) => return Ok(vm_try!(object.borrow_ref()).into_iterator()),
            target => target,
        };

        let value = vm_try!(EnvProtocolCaller.call_protocol_fn(Protocol::INTO_ITER, target, ()));
        Iterator::from_value(value)
    }

    /// Coerce into future, or convert into a future using the
    /// [Protocol::INTO_FUTURE] protocol.
    ///
    /// You must use [Vm::with] to specify which virtual machine this function
    /// is called inside.
    ///
    /// # Errors
    ///
    /// This function errors in case the provided type cannot be converted into
    /// a future without the use of a [`Vm`] and one is not provided through the
    /// environment.
    pub fn into_future(self) -> VmResult<Future> {
        let target = match self {
            Value::Future(fut) => return Ok(vm_try!(fut.take())),
            target => target,
        };

        let value = vm_try!(EnvProtocolCaller.call_protocol_fn(Protocol::INTO_FUTURE, target, ()));
        Future::from_value(value)
    }

    /// Coerce into a shared future, or convert into a future using the
    /// [Protocol::INTO_FUTURE] protocol.
    ///
    /// You must use [Vm::with] to specify which virtual machine this function
    /// is called inside.
    ///
    /// # Errors
    ///
    /// This function errors in case the provided type cannot be converted into
    /// a future without the use of a [`Vm`] and one is not provided through the
    /// environment.
    #[inline]
    pub fn into_shared_future(self) -> VmResult<Shared<Future>> {
        let target = match self {
            Value::Future(future) => return Ok(future),
            target => target,
        };

        let value = vm_try!(EnvProtocolCaller.call_protocol_fn(Protocol::INTO_FUTURE, target, ()));
        Ok(Shared::new(vm_try!(Future::from_value(value))))
    }

    /// Retrieves a human readable type name for the current value.
    ///
    /// You must use [Vm::with] to specify which virtual machine this function
    /// is called inside.
    ///
    /// # Errors
    ///
    /// This function errors in case the provided type cannot be converted into
    /// a name without the use of a [`Vm`] and one is not provided through the
    /// environment.
    pub fn into_type_name(self) -> VmResult<String> {
        let hash = Hash::instance_function(vm_try!(self.type_hash()), Protocol::INTO_TYPE_NAME);

        crate::runtime::env::with(|context, unit| {
            if let Some(name) = context.constant(hash) {
                match name {
                    ConstValue::String(s) => return Ok(s.clone()),
                    ConstValue::StaticString(s) => return Ok((*s).to_string()),
                    _ => return err(VmErrorKind::expected::<String>(name.type_info())),
                }
            }

            if let Some(name) = unit.constant(hash) {
                match name {
                    ConstValue::String(s) => return Ok(s.clone()),
                    ConstValue::StaticString(s) => return Ok((*s).to_string()),
                    _ => return err(VmErrorKind::expected::<String>(name.type_info())),
                }
            }

            Ok(vm_try!(self.type_info()).to_string())
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
    pub fn take(self) -> VmResult<Self> {
        Ok(match self {
            Self::Unit => Self::Unit,
            Self::Bool(value) => Self::Bool(value),
            Self::Byte(value) => Self::Byte(value),
            Self::Char(value) => Self::Char(value),
            Self::Integer(value) => Self::Integer(value),
            Self::Float(value) => Self::Float(value),
            Self::Type(value) => Self::Type(value),
            Self::StaticString(value) => Self::StaticString(value),
            Self::String(value) => Self::String(Shared::new(vm_try!(value.take()))),
            Self::Bytes(value) => Self::Bytes(Shared::new(vm_try!(value.take()))),
            Self::Vec(value) => Self::Vec(Shared::new(vm_try!(value.take()))),
            Self::Tuple(value) => Self::Tuple(Shared::new(vm_try!(value.take()))),
            Self::Object(value) => Self::Object(Shared::new(vm_try!(value.take()))),
            Self::Range(value) => Self::Range(Shared::new(vm_try!(value.take()))),
            Self::Future(value) => Self::Future(Shared::new(vm_try!(value.take()))),
            Self::Stream(value) => Self::Stream(Shared::new(vm_try!(value.take()))),
            Self::Generator(value) => Self::Generator(Shared::new(vm_try!(value.take()))),
            Self::GeneratorState(value) => Self::GeneratorState(Shared::new(vm_try!(value.take()))),
            Self::Option(value) => Self::Option(Shared::new(vm_try!(value.take()))),
            Self::Result(value) => Self::Result(Shared::new(vm_try!(value.take()))),
            Self::UnitStruct(value) => Self::UnitStruct(Shared::new(vm_try!(value.take()))),
            Self::TupleStruct(value) => Self::TupleStruct(Shared::new(vm_try!(value.take()))),
            Self::Struct(value) => Self::Struct(Shared::new(vm_try!(value.take()))),
            Self::Variant(value) => Self::Variant(Shared::new(vm_try!(value.take()))),
            Self::Function(value) => Self::Function(Shared::new(vm_try!(value.take()))),
            Self::Format(value) => Self::Format(value),
            Self::Iterator(value) => Self::Iterator(value),
            Self::Any(value) => Self::Any(Shared::new(vm_try!(value.take()))),
        })
    }

    /// Try to coerce value into a unit.
    #[inline]
    pub fn into_unit(self) -> VmResult<()> {
        match self {
            Value::Unit => Ok(()),
            actual => err(VmErrorKind::expected::<()>(vm_try!(actual.type_info()))),
        }
    }

    /// Try to coerce value into a boolean.
    #[inline]
    pub fn into_bool(self) -> VmResult<bool> {
        match self {
            Self::Bool(b) => Ok(b),
            actual => err(VmErrorKind::expected::<bool>(vm_try!(actual.type_info()))),
        }
    }

    /// Try to coerce value into a boolean.
    #[inline]
    pub fn as_bool(&self) -> VmResult<bool> {
        match self {
            Self::Bool(b) => Ok(*b),
            actual => err(VmErrorKind::expected::<bool>(vm_try!(actual.type_info()))),
        }
    }

    /// Try to coerce value into a byte.
    #[inline]
    pub fn into_byte(self) -> VmResult<u8> {
        match self {
            Self::Byte(b) => Ok(b),
            actual => err(VmErrorKind::expected::<u8>(vm_try!(actual.type_info()))),
        }
    }

    /// Try to coerce value into a character.
    #[inline]
    pub fn into_char(self) -> VmResult<char> {
        match self {
            Self::Char(c) => Ok(c),
            actual => err(VmErrorKind::expected::<char>(vm_try!(actual.type_info()))),
        }
    }

    /// Try to coerce value into an integer.
    #[inline]
    pub fn into_integer(self) -> VmResult<i64> {
        match self {
            Self::Integer(integer) => Ok(integer),
            actual => err(VmErrorKind::expected::<i64>(vm_try!(actual.type_info()))),
        }
    }

    /// Try to coerce value into a float.
    #[inline]
    pub fn into_float(self) -> VmResult<f64> {
        match self {
            Self::Float(float) => Ok(float),
            actual => err(VmErrorKind::expected::<f64>(vm_try!(actual.type_info()))),
        }
    }

    /// Try to coerce value into a result.
    #[inline]
    pub fn into_result(self) -> VmResult<Shared<Result<Value, Value>>> {
        match self {
            Self::Result(result) => Ok(result),
            actual => err(VmErrorKind::expected::<Result<Value, Value>>(vm_try!(
                actual.type_info()
            ))),
        }
    }

    /// Try to coerce value into a result.
    #[inline]
    pub fn as_result(&self) -> VmResult<&Shared<Result<Value, Value>>> {
        match self {
            Self::Result(result) => Ok(result),
            actual => err(VmErrorKind::expected::<Result<Value, Value>>(vm_try!(
                actual.type_info()
            ))),
        }
    }

    /// Try to coerce value into a generator.
    #[inline]
    pub fn into_generator(self) -> VmResult<Shared<Generator<Vm>>> {
        match self {
            Value::Generator(generator) => Ok(generator),
            actual => err(VmErrorKind::expected::<Generator<Vm>>(vm_try!(
                actual.type_info()
            ))),
        }
    }

    /// Try to coerce value into a stream.
    #[inline]
    pub fn into_stream(self) -> VmResult<Shared<Stream<Vm>>> {
        match self {
            Value::Stream(stream) => Ok(stream),
            actual => err(VmErrorKind::expected::<Stream<Vm>>(vm_try!(
                actual.type_info()
            ))),
        }
    }

    /// Try to coerce value into a future.
    #[inline]
    pub fn into_generator_state(self) -> VmResult<Shared<GeneratorState>> {
        match self {
            Value::GeneratorState(state) => Ok(state),
            actual => err(VmErrorKind::expected::<GeneratorState>(vm_try!(
                actual.type_info()
            ))),
        }
    }

    /// Try to coerce value into an option.
    #[inline]
    pub fn into_option(self) -> VmResult<Shared<Option<Value>>> {
        match self {
            Self::Option(option) => Ok(option),
            actual => err(VmErrorKind::expected::<Option<Value>>(vm_try!(
                actual.type_info()
            ))),
        }
    }

    /// Try to coerce value into a string.
    #[inline]
    pub fn into_string(self) -> VmResult<Shared<String>> {
        match self {
            Self::String(string) => Ok(string),
            actual => err(VmErrorKind::expected::<String>(vm_try!(actual.type_info()))),
        }
    }

    /// Try to coerce value into bytes.
    #[inline]
    pub fn into_bytes(self) -> VmResult<Shared<Bytes>> {
        match self {
            Self::Bytes(bytes) => Ok(bytes),
            actual => err(VmErrorKind::expected::<Bytes>(vm_try!(actual.type_info()))),
        }
    }

    /// Try to coerce value into a vector.
    #[inline]
    pub fn into_vec(self) -> VmResult<Shared<Vec>> {
        match self {
            Self::Vec(vec) => Ok(vec),
            actual => err(VmErrorKind::expected::<Vec>(vm_try!(actual.type_info()))),
        }
    }

    /// Try to coerce value into a tuple.
    #[inline]
    pub fn into_tuple(self) -> VmResult<Shared<Tuple>> {
        match self {
            Self::Tuple(tuple) => Ok(tuple),
            actual => err(VmErrorKind::expected::<Tuple>(vm_try!(actual.type_info()))),
        }
    }

    /// Try to coerce value into an object.
    #[inline]
    pub fn into_object(self) -> VmResult<Shared<Object>> {
        match self {
            Self::Object(object) => Ok(object),
            actual => err(VmErrorKind::expected::<Object>(vm_try!(actual.type_info()))),
        }
    }

    /// Try to coerce value into a range.
    #[inline]
    pub fn into_range(self) -> VmResult<Shared<Range>> {
        match self {
            Self::Range(object) => Ok(object),
            actual => err(VmErrorKind::expected::<Range>(vm_try!(actual.type_info()))),
        }
    }

    /// Try to coerce value into a function pointer.
    #[inline]
    pub fn into_function(self) -> VmResult<Shared<Function>> {
        match self {
            Self::Function(function) => Ok(function),
            actual => err(VmErrorKind::expected::<Function>(vm_try!(
                actual.type_info()
            ))),
        }
    }

    /// Try to coerce value into a format spec.
    #[inline]
    pub fn into_format(self) -> VmResult<Box<Format>> {
        match self {
            Value::Format(format) => Ok(format),
            actual => err(VmErrorKind::expected::<Format>(vm_try!(actual.type_info()))),
        }
    }

    /// Try to coerce value into an iterator.
    #[inline]
    pub fn into_iterator(self) -> VmResult<Shared<Iterator>> {
        match self {
            Value::Iterator(format) => Ok(format),
            actual => err(VmErrorKind::expected::<Iterator>(vm_try!(
                actual.type_info()
            ))),
        }
    }

    /// Try to coerce value into an opaque value.
    #[inline]
    pub fn into_any(self) -> VmResult<Shared<AnyObj>> {
        match self {
            Self::Any(any) => Ok(any),
            actual => err(VmErrorKind::expected_any(vm_try!(actual.type_info()))),
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
    pub fn into_any_ptr<T>(self) -> VmResult<(*const T, RawRef)>
    where
        T: Any,
    {
        match self {
            Self::Any(any) => {
                let any = vm_try!(any.internal_downcast_into_ref::<T>(AccessKind::Any));
                let (data, guard) = Ref::into_raw(any);
                Ok((data, guard))
            }
            actual => err(VmErrorKind::expected_any(vm_try!(actual.type_info()))),
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
    pub fn into_any_mut<T>(self) -> VmResult<(*mut T, RawMut)>
    where
        T: Any,
    {
        match self {
            Self::Any(any) => {
                let any = vm_try!(any.internal_downcast_into_mut::<T>(AccessKind::Any));
                let (data, guard) = Mut::into_raw(any);
                Ok((data, guard))
            }
            actual => err(VmErrorKind::expected_any(vm_try!(actual.type_info()))),
        }
    }

    /// Get the type hash for the current value.
    ///
    /// One notable feature is that the type of a variant is its container
    /// *enum*, and not the type hash of the variant itself.
    pub fn type_hash(&self) -> VmResult<Hash> {
        Ok(match self {
            Self::Unit => crate::runtime::UNIT_TYPE.hash,
            Self::Bool(..) => crate::runtime::BOOL_TYPE.hash,
            Self::Byte(..) => crate::runtime::BYTE_TYPE.hash,
            Self::Char(..) => crate::runtime::CHAR_TYPE.hash,
            Self::Integer(..) => crate::runtime::INTEGER_TYPE.hash,
            Self::Float(..) => crate::runtime::FLOAT_TYPE.hash,
            Self::StaticString(..) => crate::runtime::STRING_TYPE.hash,
            Self::String(..) => crate::runtime::STRING_TYPE.hash,
            Self::Bytes(..) => crate::runtime::BYTES_TYPE.hash,
            Self::Vec(..) => crate::runtime::VEC_TYPE.hash,
            Self::Tuple(..) => crate::runtime::TUPLE_TYPE.hash,
            Self::Object(..) => crate::runtime::OBJECT_TYPE.hash,
            Self::Range(..) => crate::runtime::RANGE_TYPE.hash,
            Self::Future(..) => crate::runtime::FUTURE_TYPE.hash,
            Self::Stream(..) => crate::runtime::STREAM_TYPE.hash,
            Self::Generator(..) => crate::runtime::GENERATOR_TYPE.hash,
            Self::GeneratorState(..) => crate::runtime::GENERATOR_STATE_TYPE.hash,
            Self::Result(..) => crate::runtime::RESULT_TYPE.hash,
            Self::Option(..) => crate::runtime::OPTION_TYPE.hash,
            Self::Function(func) => vm_try!(func.borrow_ref()).type_hash(),
            Self::Format(..) => crate::runtime::FORMAT_TYPE.hash,
            Self::Iterator(..) => crate::runtime::ITERATOR_TYPE.hash,
            Self::Type(hash) => *hash,
            Self::UnitStruct(empty) => vm_try!(empty.borrow_ref()).rtti.hash,
            Self::TupleStruct(tuple) => vm_try!(tuple.borrow_ref()).rtti.hash,
            Self::Struct(object) => vm_try!(object.borrow_ref()).rtti.hash,
            Self::Variant(variant) => vm_try!(variant.borrow_ref()).rtti().enum_hash,
            Self::Any(any) => vm_try!(any.borrow_ref()).type_hash(),
        })
    }

    /// Get the type information for the current value.
    pub fn type_info(&self) -> VmResult<TypeInfo> {
        Ok(match self {
            Self::Unit => TypeInfo::StaticType(crate::runtime::UNIT_TYPE),
            Self::Bool(..) => TypeInfo::StaticType(crate::runtime::BOOL_TYPE),
            Self::Byte(..) => TypeInfo::StaticType(crate::runtime::BYTE_TYPE),
            Self::Char(..) => TypeInfo::StaticType(crate::runtime::CHAR_TYPE),
            Self::Integer(..) => TypeInfo::StaticType(crate::runtime::INTEGER_TYPE),
            Self::Float(..) => TypeInfo::StaticType(crate::runtime::FLOAT_TYPE),
            Self::StaticString(..) => TypeInfo::StaticType(crate::runtime::STRING_TYPE),
            Self::String(..) => TypeInfo::StaticType(crate::runtime::STRING_TYPE),
            Self::Bytes(..) => TypeInfo::StaticType(crate::runtime::BYTES_TYPE),
            Self::Vec(..) => TypeInfo::StaticType(crate::runtime::VEC_TYPE),
            Self::Tuple(..) => TypeInfo::StaticType(crate::runtime::TUPLE_TYPE),
            Self::Object(..) => TypeInfo::StaticType(crate::runtime::OBJECT_TYPE),
            Self::Range(..) => TypeInfo::StaticType(crate::runtime::RANGE_TYPE),
            Self::Future(..) => TypeInfo::StaticType(crate::runtime::FUTURE_TYPE),
            Self::Stream(..) => TypeInfo::StaticType(crate::runtime::STREAM_TYPE),
            Self::Generator(..) => TypeInfo::StaticType(crate::runtime::GENERATOR_TYPE),
            Self::GeneratorState(..) => TypeInfo::StaticType(crate::runtime::GENERATOR_STATE_TYPE),
            Self::Option(..) => TypeInfo::StaticType(crate::runtime::OPTION_TYPE),
            Self::Result(..) => TypeInfo::StaticType(crate::runtime::RESULT_TYPE),
            Self::Function(..) => TypeInfo::StaticType(crate::runtime::FUNCTION_TYPE),
            Self::Format(..) => TypeInfo::StaticType(crate::runtime::FORMAT_TYPE),
            Self::Iterator(..) => TypeInfo::StaticType(crate::runtime::ITERATOR_TYPE),
            Self::Type(..) => TypeInfo::StaticType(crate::runtime::TYPE),
            Self::UnitStruct(empty) => vm_try!(empty.borrow_ref()).type_info(),
            Self::TupleStruct(tuple) => vm_try!(tuple.borrow_ref()).type_info(),
            Self::Struct(object) => vm_try!(object.borrow_ref()).type_info(),
            Self::Variant(empty) => vm_try!(empty.borrow_ref()).type_info(),
            Self::Any(any) => vm_try!(any.borrow_ref()).type_info(),
        })
    }

    /// Optimized function to test if two value pointers are deeply equal to
    /// each other.
    ///
    /// This is the basis for the eq operation (`==`).
    pub(crate) fn value_ptr_eq(vm: &mut Vm, a: &Value, b: &Value) -> VmResult<bool> {
        match (a, b) {
            (Self::Unit, Self::Unit) => return Ok(true),
            (Self::Bool(a), Self::Bool(b)) => return Ok(a == b),
            (Self::Byte(a), Self::Byte(b)) => return Ok(a == b),
            (Self::Char(a), Self::Char(b)) => return Ok(a == b),
            (Self::Integer(a), Self::Integer(b)) => return Ok(a == b),
            (Self::Float(a), Self::Float(b)) => return Ok(a == b),
            (Self::Vec(a), Self::Vec(b)) => {
                let a = vm_try!(a.borrow_ref());
                let b = vm_try!(b.borrow_ref());
                return Vec::value_ptr_eq(vm, &a, &b);
            }
            (Self::Tuple(a), Self::Tuple(b)) => {
                let a = vm_try!(a.borrow_ref());
                let b = vm_try!(b.borrow_ref());
                return Tuple::value_ptr_eq(vm, &a, &b);
            }
            (Self::Object(a), Self::Object(b)) => {
                let a = vm_try!(a.borrow_ref());
                let b = vm_try!(b.borrow_ref());
                return Object::value_ptr_eq(vm, &a, &b);
            }
            (Self::Range(a), Self::Range(b)) => {
                let a = vm_try!(a.borrow_ref());
                let b = vm_try!(b.borrow_ref());
                return Range::value_ptr_eq(vm, &a, &b);
            }
            (Self::UnitStruct(a), Self::UnitStruct(b)) => {
                if vm_try!(a.borrow_ref()).rtti.hash == vm_try!(b.borrow_ref()).rtti.hash {
                    // NB: don't get any future ideas, this must fall through to
                    // the VmError below since it's otherwise a comparison
                    // between two incompatible types.
                    //
                    // Other than that, all units are equal.
                    return Ok(true);
                }
            }
            (Self::TupleStruct(a), Self::TupleStruct(b)) => {
                let a = vm_try!(a.borrow_ref());
                let b = vm_try!(b.borrow_ref());

                if a.rtti.hash == b.rtti.hash {
                    return Tuple::value_ptr_eq(vm, &a.data, &b.data);
                }
            }
            (Self::Struct(a), Self::Struct(b)) => {
                let a = vm_try!(a.borrow_ref());
                let b = vm_try!(b.borrow_ref());

                if a.rtti.hash == b.rtti.hash {
                    return Object::value_ptr_eq(vm, &a.data, &b.data);
                }
            }
            (Self::Variant(a), Self::Variant(b)) => {
                let a = vm_try!(a.borrow_ref());
                let b = vm_try!(b.borrow_ref());

                if a.rtti().enum_hash == b.rtti().enum_hash {
                    return Variant::value_ptr_eq(vm, &a, &b);
                }
            }
            (Self::String(a), Self::String(b)) => {
                return Ok(*vm_try!(a.borrow_ref()) == *vm_try!(b.borrow_ref()));
            }
            (Self::StaticString(a), Self::String(b)) => {
                let b = vm_try!(b.borrow_ref());
                return Ok(***a == *b);
            }
            (Self::String(a), Self::StaticString(b)) => {
                let a = vm_try!(a.borrow_ref());
                return Ok(*a == ***b);
            }
            // fast string comparison: exact string slot.
            (Self::StaticString(a), Self::StaticString(b)) => {
                return Ok(***a == ***b);
            }
            (Self::Option(a), Self::Option(b)) => {
                match (&*vm_try!(a.borrow_ref()), &*vm_try!(b.borrow_ref())) {
                    (Some(a), Some(b)) => return Self::value_ptr_eq(vm, a, b),
                    (None, None) => return Ok(true),
                    _ => return Ok(false),
                }
            }
            (Self::Result(a), Self::Result(b)) => {
                match (&*vm_try!(a.borrow_ref()), &*vm_try!(b.borrow_ref())) {
                    (Result::Ok(a), Result::Ok(b)) => return Self::value_ptr_eq(vm, a, b),
                    (Result::Err(a), Result::Err(b)) => return Self::value_ptr_eq(vm, a, b),
                    _ => return Ok(false),
                }
            }
            (a, b) => match vm_try!(vm.call_instance_fn(a.clone(), Protocol::EQ, (b.clone(),))) {
                CallResult::Ok(()) => return bool::from_value(vm_try!(vm.stack_mut().pop())),
                CallResult::Unsupported(..) => {}
            },
        }

        err(VmErrorKind::from(VmErrorKind::UnsupportedBinaryOperation {
            op: "==",
            lhs: vm_try!(a.type_info()),
            rhs: vm_try!(b.type_info()),
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

                match value.string_debug(&mut s) {
                    Ok(result) => result?,
                    Err(error) => return write!(f, "{:?}", error),
                }

                f.write_str(&s)?;
            }
        }

        Result::Ok(())
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

impl ToValue for Value {
    fn to_value(self) -> VmResult<Value> {
        Ok(self)
    }
}

impl ToValue for () {
    fn to_value(self) -> VmResult<Value> {
        Ok(Value::from(()))
    }
}

macro_rules! impl_from {
    ($($variant:ident => $ty:ty),* $(,)*) => {
        $(
            impl From<$ty> for Value {
                #[inline]
                fn from(value: $ty) -> Self {
                    Self::$variant(value)
                }
            }

            impl ToValue for $ty {
                #[inline]
                fn to_value(self) -> VmResult<Value> {
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
                #[inline]
                fn from(value: $ty) -> Self {
                    Self::$variant($wrapper::new(value))
                }
            }

            impl ToValue for $ty {
                #[inline]
                fn to_value(self) -> VmResult<Value> {
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
    Stream => Shared<Stream<Vm>>,
    Generator => Shared<Generator<Vm>>,
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
                serializer.serialize_str(&string)
            }
            Value::Bytes(bytes) => {
                let bytes = bytes.borrow_ref().map_err(ser::Error::custom)?;
                serializer.serialize_bytes(&bytes)
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
            Value::TupleStruct(..) => {
                Result::Err(ser::Error::custom("cannot serialize tuple structs"))
            }
            Value::Struct(..) => {
                Result::Err(ser::Error::custom("cannot serialize objects structs"))
            }
            Value::Variant(..) => Result::Err(ser::Error::custom("cannot serialize variants")),
            Value::Result(..) => Result::Err(ser::Error::custom("cannot serialize results")),
            Value::Type(..) => Result::Err(ser::Error::custom("cannot serialize types")),
            Value::Future(..) => Result::Err(ser::Error::custom("cannot serialize futures")),
            Value::Stream(..) => Result::Err(ser::Error::custom("cannot serialize streams")),
            Value::Generator(..) => Result::Err(ser::Error::custom("cannot serialize generators")),
            Value::GeneratorState(..) => {
                Result::Err(ser::Error::custom("cannot serialize generator states"))
            }
            Value::Function(..) => {
                Result::Err(ser::Error::custom("cannot serialize function pointers"))
            }
            Value::Format(..) => {
                Result::Err(ser::Error::custom("cannot serialize format specifications"))
            }
            Value::Iterator(..) => Result::Err(ser::Error::custom("cannot serialize iterators")),
            Value::Range(..) => Result::Err(ser::Error::custom("cannot serialize ranges")),
            Value::Any(..) => Result::Err(ser::Error::custom("cannot serialize external objects")),
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
        Result::Ok(Value::String(Shared::new(value.to_owned())))
    }

    #[inline]
    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Result::Ok(Value::String(Shared::new(value)))
    }

    #[inline]
    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Result::Ok(Value::Bytes(Shared::new(Bytes::from_vec(v.to_vec()))))
    }

    #[inline]
    fn visit_byte_buf<E>(self, v: vec::Vec<u8>) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Result::Ok(Value::Bytes(Shared::new(Bytes::from_vec(v))))
    }

    #[inline]
    fn visit_i8<E>(self, v: i8) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Result::Ok(Value::Integer(v as i64))
    }

    #[inline]
    fn visit_i16<E>(self, v: i16) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Result::Ok(Value::Integer(v as i64))
    }

    #[inline]
    fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Result::Ok(Value::Integer(v as i64))
    }

    #[inline]
    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Result::Ok(Value::Integer(v))
    }

    #[inline]
    fn visit_i128<E>(self, v: i128) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Result::Ok(Value::Integer(v as i64))
    }

    #[inline]
    fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Result::Ok(Value::Integer(v as i64))
    }

    #[inline]
    fn visit_u16<E>(self, v: u16) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Result::Ok(Value::Integer(v as i64))
    }

    #[inline]
    fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Result::Ok(Value::Integer(v as i64))
    }

    #[inline]
    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Result::Ok(Value::Integer(v as i64))
    }

    #[inline]
    fn visit_u128<E>(self, v: u128) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Result::Ok(Value::Integer(v as i64))
    }

    #[inline]
    fn visit_f32<E>(self, v: f32) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Result::Ok(Value::Float(v as f64))
    }

    #[inline]
    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Result::Ok(Value::Float(v))
    }

    #[inline]
    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Result::Ok(Value::Bool(v))
    }

    #[inline]
    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Result::Ok(Value::Unit)
    }

    #[inline]
    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Result::Ok(Value::Unit)
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

        Result::Ok(Value::Vec(Shared::new(Vec::from(vec))))
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

        Result::Ok(Value::Object(Shared::new(object)))
    }
}

impl MaybeTypeOf for Value {
    #[inline]
    fn maybe_type_of() -> Option<FullTypeOf> {
        None
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
