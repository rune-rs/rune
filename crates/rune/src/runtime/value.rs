mod serde;

use core::any;
use core::borrow::Borrow;
use core::cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd};
use core::fmt;
use core::hash;
use core::ptr;

use ::rust_alloc::sync::Arc;

use crate::alloc::fmt::TryWrite;
use crate::alloc::prelude::*;
use crate::alloc::{self, String};
use crate::compile::ItemBuf;
use crate::runtime::vm::CallResult;
use crate::runtime::{
    AccessKind, AnyObj, Bytes, ConstValue, ControlFlow, EnvProtocolCaller, Format, Formatter,
    FromValue, FullTypeOf, Function, Future, Generator, GeneratorState, Iterator, MaybeTypeOf, Mut,
    Object, OwnedTuple, Protocol, ProtocolCaller, Range, RangeFrom, RangeFull, RangeInclusive,
    RangeTo, RangeToInclusive, RawMut, RawRef, Ref, Shared, Stream, ToValue, Type, TypeInfo,
    Variant, Vec, Vm, VmError, VmErrorKind, VmIntegerRepr, VmResult,
};
#[cfg(feature = "alloc")]
use crate::runtime::{Hasher, Tuple};
use crate::{Any, Hash};

use ::serde::{Deserialize, Serialize};

// Small helper function to build errors.
fn err<T, E>(error: E) -> VmResult<T>
where
    VmErrorKind: From<E>,
{
    VmResult::err(error)
}

/// A empty with a well-defined type.
pub struct EmptyStruct {
    /// The type hash of the empty.
    pub(crate) rtti: Arc<Rtti>,
}

impl EmptyStruct {
    /// Access runtime type information.
    pub fn rtti(&self) -> &Arc<Rtti> {
        &self.rtti
    }

    /// Get type info for the typed tuple.
    pub fn type_info(&self) -> TypeInfo {
        TypeInfo::Typed(self.rtti.clone())
    }
}

impl fmt::Debug for EmptyStruct {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.rtti.item)
    }
}

/// A tuple with a well-defined type.
pub struct TupleStruct {
    /// The type hash of the tuple.
    pub(crate) rtti: Arc<Rtti>,
    /// Content of the tuple.
    pub(crate) data: OwnedTuple,
}

impl TupleStruct {
    /// Access runtime type information.
    pub fn rtti(&self) -> &Arc<Rtti> {
        &self.rtti
    }

    /// Access underlying data.
    pub fn data(&self) -> &OwnedTuple {
        &self.data
    }

    /// Access underlying data mutably.
    pub fn data_mut(&mut self) -> &mut OwnedTuple {
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
        String: Borrow<Q>,
        Q: hash::Hash + Eq + Ord,
    {
        self.data.get(k)
    }

    /// Get the given mutable value by key in the object.
    pub fn get_mut<Q: ?Sized>(&mut self, k: &Q) -> Option<&mut Value>
    where
        String: Borrow<Q>,
        Q: hash::Hash + Eq + Ord,
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

impl PartialEq for VariantRtti {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
    }
}

impl Eq for VariantRtti {}

impl hash::Hash for VariantRtti {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.hash.hash(state)
    }
}

impl PartialOrd for VariantRtti {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for VariantRtti {
    fn cmp(&self, other: &Self) -> Ordering {
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

impl PartialEq for Rtti {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
    }
}

impl Eq for Rtti {}

impl hash::Hash for Rtti {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.hash.hash(state)
    }
}

impl PartialOrd for Rtti {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Rtti {
    fn cmp(&self, other: &Self) -> Ordering {
        self.hash.cmp(&other.hash)
    }
}

/// An entry on the stack.
#[derive(Clone)]
pub enum Value {
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
    Type(Type),
    /// Ordering.
    Ordering(Ordering),
    /// A UTF-8 string.
    String(Shared<String>),
    /// A byte string.
    Bytes(Shared<Bytes>),
    /// A vector containing any values.
    Vec(Shared<Vec>),
    /// The unit value.
    EmptyTuple,
    /// A tuple.
    Tuple(Shared<OwnedTuple>),
    /// An object.
    Object(Shared<Object>),
    /// A range `start..`
    RangeFrom(Shared<RangeFrom>),
    /// A full range `..`
    RangeFull(Shared<RangeFull>),
    /// A full range `start..=end`
    RangeInclusive(Shared<RangeInclusive>),
    /// A full range `..=end`
    RangeToInclusive(Shared<RangeToInclusive>),
    /// A full range `..end`
    RangeTo(Shared<RangeTo>),
    /// A range `start..end`.
    Range(Shared<Range>),
    /// A control flow indicator.
    ControlFlow(Shared<ControlFlow>),
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
    EmptyStruct(Shared<EmptyStruct>),
    /// A tuple with a well-defined type.
    TupleStruct(Shared<TupleStruct>),
    /// An struct with a well-defined type.
    Struct(Shared<Struct>),
    /// The variant of an enum.
    Variant(Shared<Variant>),
    /// A stored function pointer.
    Function(Shared<Function>),
    /// A value being formatted.
    Format(Shared<Format>),
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
    pub fn string_display(&self, f: &mut Formatter) -> VmResult<()> {
        self.string_display_with(f, &mut EnvProtocolCaller)
    }

    /// Internal impl of string_display with a customizable caller.
    pub(crate) fn string_display_with(
        &self,
        f: &mut Formatter,
        caller: &mut impl ProtocolCaller,
    ) -> VmResult<()> {
        match self {
            Value::Format(format) => {
                let format = vm_try!(format.borrow_ref());
                vm_try!(format.spec.format(&format.value, f, caller));
            }
            Value::Char(c) => {
                vm_try!(f.push(*c));
            }
            Value::String(string) => {
                vm_try!(f.push_str(&vm_try!(string.borrow_ref())));
            }
            Value::Integer(integer) => {
                let mut buffer = itoa::Buffer::new();
                vm_try!(f.push_str(buffer.format(*integer)));
            }
            Value::Float(float) => {
                let mut buffer = ryu::Buffer::new();
                vm_try!(f.push_str(buffer.format(*float)));
            }
            Value::Bool(bool) => {
                return VmResult::Ok(vm_write!(f, "{}", bool));
            }
            Value::Byte(byte) => {
                let mut buffer = itoa::Buffer::new();
                vm_try!(f.push_str(buffer.format(*byte)));
            }
            value => {
                let result = vm_try!(caller.call_protocol_fn(
                    Protocol::STRING_DISPLAY,
                    value.clone(),
                    (f,),
                ));

                return VmResult::Ok(vm_try!(<()>::from_value(result)));
            }
        }

        VmResult::Ok(())
    }

    /// Debug format the value using the [`STRING_DEBUG`] protocol.
    ///
    /// You must use [Vm::with] to specify which virtual machine this function
    /// is called inside.
    ///
    /// # Panics
    ///
    /// This function will panic if called outside of a virtual machine.
    ///
    /// [`STRING_DEBUG`]: Protocol::STRING_DEBUG
    pub fn string_debug(&self, f: &mut Formatter) -> VmResult<()> {
        self.string_debug_with(f, &mut EnvProtocolCaller)
    }

    /// Internal impl of string_debug with a customizable caller.
    pub(crate) fn string_debug_with(
        &self,
        f: &mut Formatter,
        caller: &mut impl ProtocolCaller,
    ) -> VmResult<()> {
        match self {
            Value::Bool(value) => {
                vm_write!(f, "{:?}", value);
            }
            Value::Byte(value) => {
                vm_write!(f, "{:?}", value);
            }
            Value::Char(value) => {
                vm_write!(f, "{:?}", value);
            }
            Value::Integer(value) => {
                vm_write!(f, "{:?}", value);
            }
            Value::Float(value) => {
                vm_write!(f, "{:?}", value);
            }
            Value::Type(value) => {
                vm_write!(f, "{:?}", value);
            }
            Value::String(value) => {
                vm_write!(f, "{:?}", value);
            }
            Value::Bytes(value) => {
                vm_write!(f, "{:?}", value);
            }
            Value::Vec(value) => {
                let value = vm_try!(value.borrow_ref());
                vm_try!(Vec::string_debug_with(&value, f, caller));
            }
            Value::EmptyTuple => {
                vm_write!(f, "()");
            }
            Value::Tuple(value) => {
                vm_write!(f, "{:?}", value);
            }
            Value::Object(value) => {
                vm_write!(f, "{:?}", value);
            }
            Value::RangeFrom(value) => {
                vm_write!(f, "{:?}", value);
            }
            Value::RangeFull(value) => {
                vm_write!(f, "{:?}", value);
            }
            Value::RangeInclusive(value) => {
                vm_write!(f, "{:?}", value);
            }
            Value::RangeToInclusive(value) => {
                vm_write!(f, "{:?}", value);
            }
            Value::RangeTo(value) => {
                vm_write!(f, "{:?}", value);
            }
            Value::Range(value) => {
                vm_write!(f, "{:?}", value);
            }
            Value::ControlFlow(value) => {
                let value = vm_try!(value.borrow_ref());
                vm_try!(ControlFlow::string_debug_with(&value, f, caller));
            }
            Value::Future(value) => {
                vm_write!(f, "{:?}", value);
            }
            Value::Stream(value) => {
                vm_write!(f, "{:?}", value);
            }
            Value::Generator(value) => {
                vm_write!(f, "{:?}", value);
            }
            Value::GeneratorState(value) => {
                vm_write!(f, "{:?}", value);
            }
            Value::Option(value) => {
                vm_write!(f, "{:?}", value);
            }
            Value::Result(value) => {
                vm_write!(f, "{:?}", value);
            }
            Value::EmptyStruct(value) => {
                vm_write!(f, "{:?}", value);
            }
            Value::TupleStruct(value) => {
                vm_write!(f, "{:?}", value);
            }
            Value::Struct(value) => {
                vm_write!(f, "{:?}", value);
            }
            Value::Variant(value) => {
                vm_write!(f, "{:?}", value);
            }
            Value::Function(value) => {
                vm_write!(f, "{:?}", value);
            }
            Value::Format(value) => {
                vm_write!(f, "{:?}", value);
            }
            Value::Iterator(value) => {
                vm_write!(f, "{:?}", value);
            }
            value => {
                let result =
                    vm_try!(caller.call_protocol_fn(Protocol::STRING_DEBUG, value.clone(), (f,),));

                vm_try!(<()>::from_value(result));
                return VmResult::Ok(());
            }
        };

        VmResult::Ok(())
    }

    /// Convert value into an iterator using the [`Protocol::INTO_ITER`]
    /// protocol.
    ///
    /// You must use [Vm::with] to specify which virtual machine this function
    /// is called inside.
    ///
    /// # Errors
    ///
    /// This function will error if called outside of a virtual machine context.
    pub fn into_iter(self) -> VmResult<Iterator> {
        self.into_iter_with(&mut EnvProtocolCaller)
    }

    pub(crate) fn into_iter_with(self, caller: &mut impl ProtocolCaller) -> VmResult<Iterator> {
        let target = match self {
            Value::Iterator(iterator) => return VmResult::Ok(vm_try!(iterator.take())),
            Value::Vec(vec) => {
                return VmResult::Ok(Vec::iter_ref(Ref::map(vm_try!(vec.into_ref()), |vec| {
                    &**vec
                })))
            }
            Value::Object(object) => {
                return VmResult::Ok(Object::rune_iter(vm_try!(object.into_ref())))
            }
            target => target,
        };

        let value = vm_try!(caller.call_protocol_fn(Protocol::INTO_ITER, target, ()));
        Iterator::from_value(value)
    }

    /// Coerce into a future, or convert into a future using the
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
    pub fn into_future(self) -> VmResult<Shared<Future>> {
        let target = match self {
            Value::Future(future) => return VmResult::Ok(future),
            target => target,
        };

        let value = vm_try!(EnvProtocolCaller.call_protocol_fn(Protocol::INTO_FUTURE, target, ()));
        VmResult::Ok(vm_try!(Shared::new(vm_try!(Future::from_value(value)))))
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
        let hash = Hash::associated_function(vm_try!(self.type_hash()), Protocol::INTO_TYPE_NAME);

        crate::runtime::env::with(|context, unit| {
            if let Some(name) = context.constant(hash) {
                match name {
                    ConstValue::String(s) => {
                        return VmResult::Ok(vm_try!(String::try_from(s.as_str())))
                    }
                    _ => return err(VmErrorKind::expected::<String>(name.type_info())),
                }
            }

            if let Some(name) = unit.constant(hash) {
                match name {
                    ConstValue::String(s) => {
                        return VmResult::Ok(vm_try!(String::try_from(s.as_str())))
                    }
                    _ => return err(VmErrorKind::expected::<String>(name.type_info())),
                }
            }

            VmResult::Ok(vm_try!(vm_try!(self.type_info()).try_to_string()))
        })
    }

    /// Construct a vector.
    pub fn vec(vec: alloc::Vec<Value>) -> VmResult<Self> {
        VmResult::Ok(Self::Vec(vm_try!(Shared::new(Vec::from(vec)))))
    }

    /// Construct a tuple.
    pub fn tuple(vec: alloc::Vec<Value>) -> VmResult<Self> {
        VmResult::Ok(Self::Tuple(vm_try!(Shared::new(vm_try!(
            OwnedTuple::try_from(vec)
        )))))
    }

    /// Construct an empty.
    pub fn empty_struct(rtti: Arc<Rtti>) -> VmResult<Self> {
        VmResult::Ok(Self::EmptyStruct(vm_try!(Shared::new(EmptyStruct {
            rtti
        }))))
    }

    /// Construct a typed tuple.
    pub fn tuple_struct(rtti: Arc<Rtti>, vec: alloc::Vec<Value>) -> VmResult<Self> {
        VmResult::Ok(Self::TupleStruct(vm_try!(Shared::new(TupleStruct {
            rtti,
            data: vm_try!(OwnedTuple::try_from(vec)),
        }))))
    }

    /// Construct an empty variant.
    pub fn unit_variant(rtti: Arc<VariantRtti>) -> VmResult<Self> {
        VmResult::Ok(Self::Variant(vm_try!(Shared::new(Variant::unit(rtti)))))
    }

    /// Construct a tuple variant.
    pub fn tuple_variant(rtti: Arc<VariantRtti>, vec: alloc::Vec<Value>) -> VmResult<Self> {
        VmResult::Ok(Self::Variant(vm_try!(Shared::new(Variant::tuple(
            rtti,
            vm_try!(OwnedTuple::try_from(vec))
        )))))
    }

    /// Take the interior value.
    pub fn take(self) -> VmResult<Self> {
        VmResult::Ok(match self {
            Self::Bool(value) => Self::Bool(value),
            Self::Byte(value) => Self::Byte(value),
            Self::Char(value) => Self::Char(value),
            Self::Integer(value) => Self::Integer(value),
            Self::Float(value) => Self::Float(value),
            Self::Type(value) => Self::Type(value),
            Self::Ordering(value) => Self::Ordering(value),
            Self::String(value) => Self::String(vm_try!(Shared::new(vm_try!(value.take())))),
            Self::Bytes(value) => Self::Bytes(vm_try!(Shared::new(vm_try!(value.take())))),
            Self::Vec(value) => Self::Vec(vm_try!(Shared::new(vm_try!(value.take())))),
            Self::EmptyTuple => Self::EmptyTuple,
            Self::Tuple(value) => Self::Tuple(vm_try!(Shared::new(vm_try!(value.take())))),
            Self::Object(value) => Self::Object(vm_try!(Shared::new(vm_try!(value.take())))),
            Self::RangeFrom(value) => Self::RangeFrom(vm_try!(Shared::new(vm_try!(value.take())))),
            Self::RangeFull(value) => Self::RangeFull(vm_try!(Shared::new(vm_try!(value.take())))),
            Self::RangeInclusive(value) => {
                Self::RangeInclusive(vm_try!(Shared::new(vm_try!(value.take()))))
            }
            Self::RangeToInclusive(value) => {
                Self::RangeToInclusive(vm_try!(Shared::new(vm_try!(value.take()))))
            }
            Self::RangeTo(value) => Self::RangeTo(vm_try!(Shared::new(vm_try!(value.take())))),
            Self::Range(value) => Self::Range(vm_try!(Shared::new(vm_try!(value.take())))),
            Self::ControlFlow(value) => {
                Self::ControlFlow(vm_try!(Shared::new(vm_try!(value.take()))))
            }
            Self::Future(value) => Self::Future(vm_try!(Shared::new(vm_try!(value.take())))),
            Self::Stream(value) => Self::Stream(vm_try!(Shared::new(vm_try!(value.take())))),
            Self::Generator(value) => Self::Generator(vm_try!(Shared::new(vm_try!(value.take())))),
            Self::GeneratorState(value) => {
                Self::GeneratorState(vm_try!(Shared::new(vm_try!(value.take()))))
            }
            Self::Option(value) => Self::Option(vm_try!(Shared::new(vm_try!(value.take())))),
            Self::Result(value) => Self::Result(vm_try!(Shared::new(vm_try!(value.take())))),
            Self::EmptyStruct(value) => {
                Self::EmptyStruct(vm_try!(Shared::new(vm_try!(value.take()))))
            }
            Self::TupleStruct(value) => {
                Self::TupleStruct(vm_try!(Shared::new(vm_try!(value.take()))))
            }
            Self::Struct(value) => Self::Struct(vm_try!(Shared::new(vm_try!(value.take())))),
            Self::Variant(value) => Self::Variant(vm_try!(Shared::new(vm_try!(value.take())))),
            Self::Function(value) => Self::Function(vm_try!(Shared::new(vm_try!(value.take())))),
            Self::Format(value) => Self::Format(value),
            Self::Iterator(value) => Self::Iterator(value),
            Self::Any(value) => Self::Any(vm_try!(Shared::new(vm_try!(value.take())))),
        })
    }

    /// Try to coerce value into a unit.
    #[inline]
    pub fn into_unit(self) -> VmResult<()> {
        match self {
            Value::EmptyTuple => VmResult::Ok(()),
            actual => err(VmErrorKind::expected::<()>(vm_try!(actual.type_info()))),
        }
    }

    /// Try to coerce value into a boolean.
    #[inline]
    pub fn as_bool(&self) -> VmResult<bool> {
        match self {
            Self::Bool(b) => VmResult::Ok(*b),
            actual => err(VmErrorKind::expected::<bool>(vm_try!(actual.type_info()))),
        }
    }

    /// Try to coerce value into a boolean.
    #[inline]
    pub fn into_bool(self) -> VmResult<bool> {
        match self {
            Self::Bool(b) => VmResult::Ok(b),
            actual => err(VmErrorKind::expected::<bool>(vm_try!(actual.type_info()))),
        }
    }

    /// Try to coerce value into a byte.
    #[inline]
    pub fn as_byte(&self) -> VmResult<u8> {
        match self {
            Self::Byte(b) => VmResult::Ok(*b),
            actual => err(VmErrorKind::expected::<u8>(vm_try!(actual.type_info()))),
        }
    }

    /// Try to coerce value into a byte.
    #[inline]
    pub fn into_byte(self) -> VmResult<u8> {
        match self {
            Self::Byte(b) => VmResult::Ok(b),
            actual => err(VmErrorKind::expected::<u8>(vm_try!(actual.type_info()))),
        }
    }

    /// Try to coerce value into a character.
    #[inline]
    pub fn as_char(&self) -> VmResult<char> {
        match self {
            Self::Char(c) => VmResult::Ok(*c),
            actual => err(VmErrorKind::expected::<char>(vm_try!(actual.type_info()))),
        }
    }

    /// Try to coerce value into a character.
    #[inline]
    pub fn into_char(self) -> VmResult<char> {
        match self {
            Self::Char(c) => VmResult::Ok(c),
            actual => err(VmErrorKind::expected::<char>(vm_try!(actual.type_info()))),
        }
    }

    /// Try to coerce value into an integer.
    #[inline]
    pub fn as_integer(&self) -> VmResult<i64> {
        match self {
            Self::Integer(integer) => VmResult::Ok(*integer),
            actual => err(VmErrorKind::expected::<i64>(vm_try!(actual.type_info()))),
        }
    }

    /// Try to coerce value into an integer.
    #[inline]
    pub fn into_integer(self) -> VmResult<i64> {
        match self {
            Self::Integer(integer) => VmResult::Ok(integer),
            actual => err(VmErrorKind::expected::<i64>(vm_try!(actual.type_info()))),
        }
    }

    /// Try to coerce value into a float.
    #[inline]
    pub fn as_float(&self) -> VmResult<f64> {
        match self {
            Self::Float(float) => VmResult::Ok(*float),
            actual => err(VmErrorKind::expected::<f64>(vm_try!(actual.type_info()))),
        }
    }

    /// Try to coerce value into a float.
    #[inline]
    pub fn into_float(self) -> VmResult<f64> {
        match self {
            Self::Float(float) => VmResult::Ok(float),
            actual => err(VmErrorKind::expected::<f64>(vm_try!(actual.type_info()))),
        }
    }

    /// Try to coerce value into a type.
    #[inline]
    pub fn as_type(&self) -> VmResult<Type> {
        match self {
            Self::Type(ty) => VmResult::Ok(*ty),
            actual => err(VmErrorKind::expected::<Type>(vm_try!(actual.type_info()))),
        }
    }

    /// Try to coerce value into a type.
    #[inline]
    pub fn into_type(self) -> VmResult<Type> {
        match self {
            Self::Type(ty) => VmResult::Ok(ty),
            actual => err(VmErrorKind::expected::<Type>(vm_try!(actual.type_info()))),
        }
    }

    /// Try to coerce value into a usize.
    #[inline]
    pub fn as_usize(&self) -> VmResult<usize> {
        self.try_as_integer()
    }

    /// Try to coerce value into a usize.
    #[inline]
    pub fn into_usize(self) -> VmResult<usize> {
        self.try_into_integer()
    }

    /// Try to coerce value into an [Ordering].
    #[inline]
    pub fn as_ordering(&self) -> VmResult<Ordering> {
        match self {
            Self::Ordering(ty) => VmResult::Ok(*ty),
            actual => err(VmErrorKind::expected::<Ordering>(vm_try!(
                actual.type_info()
            ))),
        }
    }

    /// Try to coerce value into an [Ordering].
    #[inline]
    pub fn into_ordering(self) -> VmResult<Ordering> {
        match self {
            Self::Ordering(ty) => VmResult::Ok(ty),
            actual => err(VmErrorKind::expected::<Ordering>(vm_try!(
                actual.type_info()
            ))),
        }
    }

    /// Try to coerce value into a result.
    #[inline]
    pub fn into_result(self) -> VmResult<Shared<Result<Value, Value>>> {
        match self {
            Self::Result(result) => VmResult::Ok(result),
            actual => err(VmErrorKind::expected::<Result<Value, Value>>(vm_try!(
                actual.type_info()
            ))),
        }
    }

    /// Try to coerce value into a result.
    #[inline]
    pub fn as_result(&self) -> VmResult<&Shared<Result<Value, Value>>> {
        match self {
            Self::Result(result) => VmResult::Ok(result),
            actual => err(VmErrorKind::expected::<Result<Value, Value>>(vm_try!(
                actual.type_info()
            ))),
        }
    }

    /// Try to coerce value into a generator.
    #[inline]
    pub fn into_generator(self) -> VmResult<Shared<Generator<Vm>>> {
        match self {
            Value::Generator(generator) => VmResult::Ok(generator),
            actual => err(VmErrorKind::expected::<Generator<Vm>>(vm_try!(
                actual.type_info()
            ))),
        }
    }

    /// Try to coerce value into a stream.
    #[inline]
    pub fn into_stream(self) -> VmResult<Shared<Stream<Vm>>> {
        match self {
            Value::Stream(stream) => VmResult::Ok(stream),
            actual => err(VmErrorKind::expected::<Stream<Vm>>(vm_try!(
                actual.type_info()
            ))),
        }
    }

    /// Try to coerce value into a future.
    #[inline]
    pub fn into_generator_state(self) -> VmResult<Shared<GeneratorState>> {
        match self {
            Value::GeneratorState(state) => VmResult::Ok(state),
            actual => err(VmErrorKind::expected::<GeneratorState>(vm_try!(
                actual.type_info()
            ))),
        }
    }

    /// Try to coerce value into an option.
    #[inline]
    pub fn into_option(self) -> VmResult<Shared<Option<Value>>> {
        match self {
            Self::Option(option) => VmResult::Ok(option),
            actual => err(VmErrorKind::expected::<Option<Value>>(vm_try!(
                actual.type_info()
            ))),
        }
    }

    /// Try to coerce value into a string.
    #[inline]
    pub fn into_string(self) -> VmResult<Shared<String>> {
        match self {
            Self::String(string) => VmResult::Ok(string),
            actual => err(VmErrorKind::expected::<String>(vm_try!(actual.type_info()))),
        }
    }

    /// Try to coerce value into bytes.
    #[inline]
    pub fn into_bytes(self) -> VmResult<Shared<Bytes>> {
        match self {
            Self::Bytes(bytes) => VmResult::Ok(bytes),
            actual => err(VmErrorKind::expected::<Bytes>(vm_try!(actual.type_info()))),
        }
    }

    /// Try to coerce value into a vector.
    #[inline]
    pub fn into_vec(self) -> VmResult<Shared<Vec>> {
        match self {
            Self::Vec(vec) => VmResult::Ok(vec),
            actual => err(VmErrorKind::expected::<Vec>(vm_try!(actual.type_info()))),
        }
    }

    /// Try to coerce value into a tuple.
    #[inline]
    pub fn into_tuple(self) -> VmResult<Shared<OwnedTuple>> {
        match self {
            Self::EmptyTuple => VmResult::Ok(vm_try!(Shared::new(OwnedTuple::new()))),
            Self::Tuple(tuple) => VmResult::Ok(tuple),
            actual => err(VmErrorKind::expected::<OwnedTuple>(vm_try!(
                actual.type_info()
            ))),
        }
    }

    /// Try to coerce value into an object.
    #[inline]
    pub fn into_object(self) -> VmResult<Shared<Object>> {
        match self {
            Self::Object(object) => VmResult::Ok(object),
            actual => err(VmErrorKind::expected::<Object>(vm_try!(actual.type_info()))),
        }
    }

    /// Try to coerce value into a [`RangeFrom`].
    #[inline]
    pub fn into_range_from(self) -> VmResult<Shared<RangeFrom>> {
        match self {
            Self::RangeFrom(object) => VmResult::Ok(object),
            actual => err(VmErrorKind::expected::<RangeFrom>(vm_try!(
                actual.type_info()
            ))),
        }
    }

    /// Try to coerce value into a [`RangeFull`].
    #[inline]
    pub fn into_range_full(self) -> VmResult<Shared<RangeFull>> {
        match self {
            Self::RangeFull(object) => VmResult::Ok(object),
            actual => err(VmErrorKind::expected::<RangeFull>(vm_try!(
                actual.type_info()
            ))),
        }
    }

    /// Try to coerce value into a [`RangeToInclusive`].
    #[inline]
    pub fn into_range_to_inclusive(self) -> VmResult<Shared<RangeToInclusive>> {
        match self {
            Self::RangeToInclusive(object) => VmResult::Ok(object),
            actual => err(VmErrorKind::expected::<RangeToInclusive>(vm_try!(
                actual.type_info()
            ))),
        }
    }

    /// Try to coerce value into a [`RangeInclusive`].
    #[inline]
    pub fn into_range_inclusive(self) -> VmResult<Shared<RangeInclusive>> {
        match self {
            Self::RangeInclusive(object) => VmResult::Ok(object),
            actual => err(VmErrorKind::expected::<RangeInclusive>(vm_try!(
                actual.type_info()
            ))),
        }
    }

    /// Try to coerce value into a [`RangeTo`].
    #[inline]
    pub fn into_range_to(self) -> VmResult<Shared<RangeTo>> {
        match self {
            Self::RangeTo(object) => VmResult::Ok(object),
            actual => err(VmErrorKind::expected::<RangeTo>(
                vm_try!(actual.type_info()),
            )),
        }
    }

    /// Try to coerce value into a [`Range`].
    #[inline]
    pub fn into_range(self) -> VmResult<Shared<Range>> {
        match self {
            Self::Range(object) => VmResult::Ok(object),
            actual => err(VmErrorKind::expected::<Range>(vm_try!(actual.type_info()))),
        }
    }

    /// Try to coerce value into a [`ControlFlow`].
    #[inline]
    pub fn into_control_flow(self) -> VmResult<Shared<ControlFlow>> {
        match self {
            Self::ControlFlow(object) => VmResult::Ok(object),
            actual => err(VmErrorKind::expected::<ControlFlow>(vm_try!(
                actual.type_info()
            ))),
        }
    }

    /// Try to coerce value into a function pointer.
    #[inline]
    pub fn into_function(self) -> VmResult<Shared<Function>> {
        match self {
            Self::Function(function) => VmResult::Ok(function),
            actual => err(VmErrorKind::expected::<Function>(vm_try!(
                actual.type_info()
            ))),
        }
    }

    /// Try to coerce value into a format spec.
    #[inline]
    pub fn into_format(self) -> VmResult<Shared<Format>> {
        match self {
            Value::Format(format) => VmResult::Ok(format),
            actual => err(VmErrorKind::expected::<Format>(vm_try!(actual.type_info()))),
        }
    }

    /// Try to coerce value into an iterator.
    #[inline]
    pub fn into_iterator(self) -> VmResult<Shared<Iterator>> {
        match self {
            Value::Iterator(format) => VmResult::Ok(format),
            actual => err(VmErrorKind::expected::<Iterator>(vm_try!(
                actual.type_info()
            ))),
        }
    }

    /// Try to coerce value into an opaque value.
    #[inline]
    pub fn into_any(self) -> VmResult<Shared<AnyObj>> {
        match self {
            Self::Any(any) => VmResult::Ok(any),
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
    pub fn into_any_ptr<T>(self) -> VmResult<(ptr::NonNull<T>, RawRef)>
    where
        T: Any,
    {
        match self {
            Self::Any(any) => {
                let any = vm_try!(any.internal_downcast_into_ref::<T>(AccessKind::Any));
                let (data, guard) = Ref::into_raw(any);
                VmResult::Ok((data, guard))
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
    pub fn into_any_mut<T>(self) -> VmResult<(ptr::NonNull<T>, RawMut)>
    where
        T: Any,
    {
        match self {
            Self::Any(any) => {
                let any = vm_try!(any.internal_downcast_into_mut::<T>(AccessKind::Any));
                let (data, guard) = Mut::into_raw(any);
                VmResult::Ok((data, guard))
            }
            actual => err(VmErrorKind::expected_any(vm_try!(actual.type_info()))),
        }
    }

    /// Get the type hash for the current value.
    ///
    /// One notable feature is that the type of a variant is its container
    /// *enum*, and not the type hash of the variant itself.
    pub fn type_hash(&self) -> Result<Hash, VmError> {
        Ok(match self {
            Self::Bool(..) => crate::runtime::static_type::BOOL_TYPE.hash,
            Self::Byte(..) => crate::runtime::static_type::BYTE_TYPE.hash,
            Self::Char(..) => crate::runtime::static_type::CHAR_TYPE.hash,
            Self::Integer(..) => crate::runtime::static_type::INTEGER_TYPE.hash,
            Self::Float(..) => crate::runtime::static_type::FLOAT_TYPE.hash,
            Self::Type(..) => crate::runtime::static_type::TYPE.hash,
            Self::Ordering(..) => crate::runtime::static_type::ORDERING_TYPE.hash,
            Self::String(..) => crate::runtime::static_type::STRING_TYPE.hash,
            Self::Bytes(..) => crate::runtime::static_type::BYTES_TYPE.hash,
            Self::Vec(..) => crate::runtime::static_type::VEC_TYPE.hash,
            Self::EmptyTuple => crate::runtime::static_type::TUPLE_TYPE.hash,
            Self::Tuple(..) => crate::runtime::static_type::TUPLE_TYPE.hash,
            Self::Object(..) => crate::runtime::static_type::OBJECT_TYPE.hash,
            Self::RangeFrom(..) => crate::runtime::static_type::RANGE_FROM_TYPE.hash,
            Self::RangeFull(..) => crate::runtime::static_type::RANGE_FULL_TYPE.hash,
            Self::RangeInclusive(..) => crate::runtime::static_type::RANGE_INCLUSIVE_TYPE.hash,
            Self::RangeToInclusive(..) => crate::runtime::static_type::RANGE_TO_INCLUSIVE_TYPE.hash,
            Self::RangeTo(..) => crate::runtime::static_type::RANGE_TO_TYPE.hash,
            Self::Range(..) => crate::runtime::static_type::RANGE_TYPE.hash,
            Self::ControlFlow(..) => crate::runtime::static_type::CONTROL_FLOW_TYPE.hash,
            Self::Future(..) => crate::runtime::static_type::FUTURE_TYPE.hash,
            Self::Stream(..) => crate::runtime::static_type::STREAM_TYPE.hash,
            Self::Generator(..) => crate::runtime::static_type::GENERATOR_TYPE.hash,
            Self::GeneratorState(..) => crate::runtime::static_type::GENERATOR_STATE_TYPE.hash,
            Self::Result(..) => crate::runtime::static_type::RESULT_TYPE.hash,
            Self::Option(..) => crate::runtime::static_type::OPTION_TYPE.hash,
            Self::Function(..) => crate::runtime::static_type::FUNCTION_TYPE.hash,
            Self::Format(..) => crate::runtime::static_type::FORMAT_TYPE.hash,
            Self::Iterator(..) => crate::runtime::static_type::ITERATOR_TYPE.hash,
            Self::EmptyStruct(empty) => empty.borrow_ref()?.rtti.hash,
            Self::TupleStruct(tuple) => tuple.borrow_ref()?.rtti.hash,
            Self::Struct(object) => object.borrow_ref()?.rtti.hash,
            Self::Variant(variant) => variant.borrow_ref()?.rtti().enum_hash,
            Self::Any(any) => any.borrow_ref()?.type_hash(),
        })
    }

    /// Get the type information for the current value.
    pub fn type_info(&self) -> VmResult<TypeInfo> {
        VmResult::Ok(match self {
            Self::Bool(..) => TypeInfo::StaticType(crate::runtime::static_type::BOOL_TYPE),
            Self::Byte(..) => TypeInfo::StaticType(crate::runtime::static_type::BYTE_TYPE),
            Self::Char(..) => TypeInfo::StaticType(crate::runtime::static_type::CHAR_TYPE),
            Self::Integer(..) => TypeInfo::StaticType(crate::runtime::static_type::INTEGER_TYPE),
            Self::Float(..) => TypeInfo::StaticType(crate::runtime::static_type::FLOAT_TYPE),
            Self::Type(..) => TypeInfo::StaticType(crate::runtime::static_type::TYPE),
            Self::Ordering(..) => TypeInfo::StaticType(crate::runtime::static_type::ORDERING_TYPE),
            Self::String(..) => TypeInfo::StaticType(crate::runtime::static_type::STRING_TYPE),
            Self::Bytes(..) => TypeInfo::StaticType(crate::runtime::static_type::BYTES_TYPE),
            Self::Vec(..) => TypeInfo::StaticType(crate::runtime::static_type::VEC_TYPE),
            Self::EmptyTuple => TypeInfo::StaticType(crate::runtime::static_type::TUPLE_TYPE),
            Self::Tuple(..) => TypeInfo::StaticType(crate::runtime::static_type::TUPLE_TYPE),
            Self::Object(..) => TypeInfo::StaticType(crate::runtime::static_type::OBJECT_TYPE),
            Self::RangeFrom(..) => {
                TypeInfo::StaticType(crate::runtime::static_type::RANGE_FROM_TYPE)
            }
            Self::RangeFull(..) => {
                TypeInfo::StaticType(crate::runtime::static_type::RANGE_FULL_TYPE)
            }
            Self::RangeInclusive(..) => {
                TypeInfo::StaticType(crate::runtime::static_type::RANGE_INCLUSIVE_TYPE)
            }
            Self::RangeToInclusive(..) => {
                TypeInfo::StaticType(crate::runtime::static_type::RANGE_TO_INCLUSIVE_TYPE)
            }
            Self::RangeTo(..) => TypeInfo::StaticType(crate::runtime::static_type::RANGE_TO_TYPE),
            Self::Range(..) => TypeInfo::StaticType(crate::runtime::static_type::RANGE_TYPE),
            Self::ControlFlow(..) => {
                TypeInfo::StaticType(crate::runtime::static_type::CONTROL_FLOW_TYPE)
            }
            Self::Future(..) => TypeInfo::StaticType(crate::runtime::static_type::FUTURE_TYPE),
            Self::Stream(..) => TypeInfo::StaticType(crate::runtime::static_type::STREAM_TYPE),
            Self::Generator(..) => {
                TypeInfo::StaticType(crate::runtime::static_type::GENERATOR_TYPE)
            }
            Self::GeneratorState(..) => {
                TypeInfo::StaticType(crate::runtime::static_type::GENERATOR_STATE_TYPE)
            }
            Self::Option(..) => TypeInfo::StaticType(crate::runtime::static_type::OPTION_TYPE),
            Self::Result(..) => TypeInfo::StaticType(crate::runtime::static_type::RESULT_TYPE),
            Self::Function(..) => TypeInfo::StaticType(crate::runtime::static_type::FUNCTION_TYPE),
            Self::Format(..) => TypeInfo::StaticType(crate::runtime::static_type::FORMAT_TYPE),
            Self::Iterator(..) => TypeInfo::StaticType(crate::runtime::static_type::ITERATOR_TYPE),
            Self::EmptyStruct(empty) => vm_try!(empty.borrow_ref()).type_info(),
            Self::TupleStruct(tuple) => vm_try!(tuple.borrow_ref()).type_info(),
            Self::Struct(object) => vm_try!(object.borrow_ref()).type_info(),
            Self::Variant(empty) => vm_try!(empty.borrow_ref()).type_info(),
            Self::Any(any) => vm_try!(any.borrow_ref()).type_info(),
        })
    }

    /// Perform a partial equality test between two values.
    ///
    /// This is the basis for the eq operation (`partial_eq` / '==').
    ///
    /// External types will use the [`Protocol::PARTIAL_EQ`] protocol when
    /// invoked through this function.
    ///
    /// # Errors
    ///
    /// This function will error if called outside of a virtual machine context.
    pub fn partial_eq(a: &Value, b: &Value) -> VmResult<bool> {
        Value::partial_eq_with(a, b, &mut EnvProtocolCaller)
    }

    /// Perform a total equality test between two values.
    ///
    /// This is the basis for the eq operation (`partial_eq` / '==').
    pub(crate) fn partial_eq_with(
        a: &Value,
        b: &Value,
        caller: &mut impl ProtocolCaller,
    ) -> VmResult<bool> {
        match (a, b) {
            (Self::Bool(a), Self::Bool(b)) => return VmResult::Ok(a == b),
            (Self::Byte(a), Self::Byte(b)) => return VmResult::Ok(a == b),
            (Self::Char(a), Self::Char(b)) => return VmResult::Ok(a == b),
            (Self::Integer(a), Self::Integer(b)) => return VmResult::Ok(a == b),
            (Self::Float(a), Self::Float(b)) => return VmResult::Ok(a == b),
            (Self::Type(a), Self::Type(b)) => return VmResult::Ok(a == b),
            (Self::Bytes(a), Self::Bytes(b)) => {
                let a = vm_try!(a.borrow_ref());
                let b = vm_try!(b.borrow_ref());
                return VmResult::Ok(*a == *b);
            }
            (Self::Vec(a), b) => {
                let a = vm_try!(a.borrow_ref());
                return Vec::partial_eq_with(&a, b.clone(), caller);
            }
            (Self::EmptyTuple, Self::EmptyTuple) => return VmResult::Ok(true),
            (Self::Tuple(a), b) => {
                let a = vm_try!(a.borrow_ref());
                return Vec::partial_eq_with(&a, b.clone(), caller);
            }
            (Self::Object(a), b) => {
                let a = vm_try!(a.borrow_ref());
                return Object::partial_eq_with(&a, b.clone(), caller);
            }
            (Self::RangeFrom(a), Self::RangeFrom(b)) => {
                let a = vm_try!(a.borrow_ref());
                let b = vm_try!(b.borrow_ref());
                return RangeFrom::partial_eq_with(&a, &b, caller);
            }
            (Self::RangeFull(a), Self::RangeFull(b)) => {
                let a = vm_try!(a.borrow_ref());
                let b = vm_try!(b.borrow_ref());
                return RangeFull::partial_eq_with(&a, &b, caller);
            }
            (Self::RangeInclusive(a), Self::RangeInclusive(b)) => {
                let a = vm_try!(a.borrow_ref());
                let b = vm_try!(b.borrow_ref());
                return RangeInclusive::partial_eq_with(&a, &b, caller);
            }
            (Self::RangeToInclusive(a), Self::RangeToInclusive(b)) => {
                let a = vm_try!(a.borrow_ref());
                let b = vm_try!(b.borrow_ref());
                return RangeToInclusive::partial_eq_with(&a, &b, caller);
            }
            (Self::RangeTo(a), Self::RangeTo(b)) => {
                let a = vm_try!(a.borrow_ref());
                let b = vm_try!(b.borrow_ref());
                return RangeTo::partial_eq_with(&a, &b, caller);
            }
            (Self::Range(a), Self::Range(b)) => {
                let a = vm_try!(a.borrow_ref());
                let b = vm_try!(b.borrow_ref());
                return Range::partial_eq_with(&a, &b, caller);
            }
            (Self::ControlFlow(a), Self::ControlFlow(b)) => {
                let a = vm_try!(a.borrow_ref());
                let b = vm_try!(b.borrow_ref());
                return ControlFlow::partial_eq_with(&a, &b, caller);
            }
            (Self::EmptyStruct(a), Self::EmptyStruct(b)) => {
                if vm_try!(a.borrow_ref()).rtti.hash == vm_try!(b.borrow_ref()).rtti.hash {
                    // NB: don't get any future ideas, this must fall through to
                    // the VmError below since it's otherwise a comparison
                    // between two incompatible types.
                    //
                    // Other than that, all units are equal.
                    return VmResult::Ok(true);
                }
            }
            (Self::TupleStruct(a), Self::TupleStruct(b)) => {
                let a = vm_try!(a.borrow_ref());
                let b = vm_try!(b.borrow_ref());

                if a.rtti.hash == b.rtti.hash {
                    return Vec::eq_with(&a.data, &b.data, Value::partial_eq_with, caller);
                }
            }
            (Self::Struct(a), Self::Struct(b)) => {
                let a = vm_try!(a.borrow_ref());
                let b = vm_try!(b.borrow_ref());

                if a.rtti.hash == b.rtti.hash {
                    return Object::eq_with(&a.data, &b.data, Value::partial_eq_with, caller);
                }
            }
            (Self::Variant(a), Self::Variant(b)) => {
                let a = vm_try!(a.borrow_ref());
                let b = vm_try!(b.borrow_ref());

                if a.rtti().enum_hash == b.rtti().enum_hash {
                    return Variant::partial_eq_with(&a, &b, caller);
                }
            }
            (Self::String(a), Self::String(b)) => {
                return VmResult::Ok(*vm_try!(a.borrow_ref()) == *vm_try!(b.borrow_ref()));
            }
            (Self::Option(a), Self::Option(b)) => {
                match (&*vm_try!(a.borrow_ref()), &*vm_try!(b.borrow_ref())) {
                    (Some(a), Some(b)) => return Self::partial_eq_with(a, b, caller),
                    (None, None) => return VmResult::Ok(true),
                    _ => return VmResult::Ok(false),
                }
            }
            (Self::Result(a), Self::Result(b)) => {
                match (&*vm_try!(a.borrow_ref()), &*vm_try!(b.borrow_ref())) {
                    (Ok(a), Ok(b)) => return Self::partial_eq_with(a, b, caller),
                    (Err(a), Err(b)) => return Self::partial_eq_with(a, b, caller),
                    _ => return VmResult::Ok(false),
                }
            }
            (a, b) => {
                match vm_try!(caller.try_call_protocol_fn(
                    Protocol::PARTIAL_EQ,
                    a.clone(),
                    (b.clone(),)
                )) {
                    CallResult::Ok(value) => return bool::from_value(value),
                    CallResult::Unsupported(..) => {}
                }
            }
        }

        err(VmErrorKind::UnsupportedBinaryOperation {
            op: "partial_eq",
            lhs: vm_try!(a.type_info()),
            rhs: vm_try!(b.type_info()),
        })
    }

    /// Hash the current value.
    #[cfg(feature = "alloc")]
    pub fn hash(&self, hasher: &mut Hasher) -> VmResult<()> {
        self.hash_with(hasher, &mut EnvProtocolCaller)
    }

    /// Hash the current value.
    pub(crate) fn hash_with(
        &self,
        hasher: &mut Hasher,
        caller: &mut impl ProtocolCaller,
    ) -> VmResult<()> {
        match self {
            Value::Integer(value) => {
                hasher.write_i64(*value);
                return VmResult::Ok(());
            }
            Value::Byte(value) => {
                hasher.write_u8(*value);
                return VmResult::Ok(());
            }
            // Care must be taken whan hashing floats, to ensure that `hash(v1)
            // === hash(v2)` if `eq(v1) === eq(v2)`. Hopefully we accomplish
            // this by rejecting NaNs and rectifying subnormal values of zero.
            Value::Float(value) => {
                if value.is_nan() {
                    return VmResult::err(VmErrorKind::IllegalFloatOperation { value: *value });
                }

                let zero = *value == 0.0;
                hasher.write_f64((zero as u8 as f64) * 0.0 + (!zero as u8 as f64) * *value);
                return VmResult::Ok(());
            }
            Value::String(string) => {
                let string = vm_try!(string.borrow_ref());
                hasher.write_str(&string);
                return VmResult::Ok(());
            }
            Value::Bytes(bytes) => {
                let bytes = vm_try!(bytes.borrow_ref());
                hasher.write(&bytes);
                return VmResult::Ok(());
            }
            Value::Tuple(tuple) => {
                let tuple = vm_try!(tuple.borrow_ref());
                return Tuple::hash_with(&tuple, hasher, caller);
            }
            Value::Vec(vec) => {
                let vec = vm_try!(vec.borrow_ref());
                return Vec::hash_with(&vec, hasher, caller);
            }
            value => {
                match vm_try!(caller.try_call_protocol_fn(Protocol::HASH, value.clone(), (hasher,)))
                {
                    CallResult::Ok(value) => return <()>::from_value(value),
                    CallResult::Unsupported(..) => {}
                }
            }
        }

        err(VmErrorKind::UnsupportedUnaryOperation {
            op: "hash",
            operand: vm_try!(self.type_info()),
        })
    }

    /// Perform a total equality test between two values.
    ///
    /// This is the basis for the eq operation (`==`).
    ///
    /// External types will use the [`Protocol::EQ`] protocol when invoked
    /// through this function.
    ///
    /// # Errors
    ///
    /// This function will error if called outside of a virtual machine context.
    pub fn eq(&self, b: &Value) -> VmResult<bool> {
        self.eq_with(b, &mut EnvProtocolCaller)
    }

    /// Perform a total equality test between two values.
    ///
    /// This is the basis for the eq operation (`==`).
    pub(crate) fn eq_with(&self, b: &Value, caller: &mut impl ProtocolCaller) -> VmResult<bool> {
        match (self, b) {
            (Self::Bool(a), Self::Bool(b)) => return VmResult::Ok(a == b),
            (Self::Byte(a), Self::Byte(b)) => return VmResult::Ok(a == b),
            (Self::Char(a), Self::Char(b)) => return VmResult::Ok(a == b),
            (Self::Float(a), Self::Float(b)) => {
                if let Some(ordering) = a.partial_cmp(b) {
                    return VmResult::Ok(matches!(ordering, Ordering::Equal));
                }

                return VmResult::err(VmErrorKind::IllegalFloatComparison { lhs: *a, rhs: *b });
            }
            (Self::Integer(a), Self::Integer(b)) => return VmResult::Ok(a == b),
            (Self::Type(a), Self::Type(b)) => return VmResult::Ok(a == b),
            (Self::Bytes(a), Self::Bytes(b)) => {
                let a = vm_try!(a.borrow_ref());
                let b = vm_try!(b.borrow_ref());
                return VmResult::Ok(*a == *b);
            }
            (Self::Vec(a), Self::Vec(b)) => {
                let a = vm_try!(a.borrow_ref());
                let b = vm_try!(b.borrow_ref());
                return Vec::eq_with(&a, &b, Value::eq_with, caller);
            }
            (Self::EmptyTuple, Self::EmptyTuple) => return VmResult::Ok(true),
            (Self::Tuple(a), Self::Tuple(b)) => {
                let a = vm_try!(a.borrow_ref());
                let b = vm_try!(b.borrow_ref());
                return Vec::eq_with(&a, &b, Value::eq_with, caller);
            }
            (Self::Object(a), Self::Object(b)) => {
                let a = vm_try!(a.borrow_ref());
                let b = vm_try!(b.borrow_ref());
                return Object::eq_with(&a, &b, Value::eq_with, caller);
            }
            (Self::RangeFrom(a), Self::RangeFrom(b)) => {
                let a = vm_try!(a.borrow_ref());
                let b = vm_try!(b.borrow_ref());
                return RangeFrom::eq_with(&a, &b, caller);
            }
            (Self::RangeFull(a), Self::RangeFull(b)) => {
                let a = vm_try!(a.borrow_ref());
                let b = vm_try!(b.borrow_ref());
                return RangeFull::eq_with(&a, &b, caller);
            }
            (Self::RangeInclusive(a), Self::RangeInclusive(b)) => {
                let a = vm_try!(a.borrow_ref());
                let b = vm_try!(b.borrow_ref());
                return RangeInclusive::eq_with(&a, &b, caller);
            }
            (Self::RangeToInclusive(a), Self::RangeToInclusive(b)) => {
                let a = vm_try!(a.borrow_ref());
                let b = vm_try!(b.borrow_ref());
                return RangeToInclusive::eq_with(&a, &b, caller);
            }
            (Self::RangeTo(a), Self::RangeTo(b)) => {
                let a = vm_try!(a.borrow_ref());
                let b = vm_try!(b.borrow_ref());
                return RangeTo::eq_with(&a, &b, caller);
            }
            (Self::Range(a), Self::Range(b)) => {
                let a = vm_try!(a.borrow_ref());
                let b = vm_try!(b.borrow_ref());
                return Range::eq_with(&a, &b, caller);
            }
            (Self::ControlFlow(a), Self::ControlFlow(b)) => {
                let a = vm_try!(a.borrow_ref());
                let b = vm_try!(b.borrow_ref());
                return ControlFlow::eq_with(&a, &b, caller);
            }
            (Self::EmptyStruct(a), Self::EmptyStruct(b)) => {
                if vm_try!(a.borrow_ref()).rtti.hash == vm_try!(b.borrow_ref()).rtti.hash {
                    // NB: don't get any future ideas, this must fall through to
                    // the VmError below since it's otherwise a comparison
                    // between two incompatible types.
                    //
                    // Other than that, all units are equal.
                    return VmResult::Ok(true);
                }
            }
            (Self::TupleStruct(a), Self::TupleStruct(b)) => {
                let a = vm_try!(a.borrow_ref());
                let b = vm_try!(b.borrow_ref());

                if a.rtti.hash == b.rtti.hash {
                    return Vec::eq_with(&a.data, &b.data, Value::eq_with, caller);
                }
            }
            (Self::Struct(a), Self::Struct(b)) => {
                let a = vm_try!(a.borrow_ref());
                let b = vm_try!(b.borrow_ref());

                if a.rtti.hash == b.rtti.hash {
                    return Object::eq_with(&a.data, &b.data, Value::eq_with, caller);
                }
            }
            (Self::Variant(a), Self::Variant(b)) => {
                let a = vm_try!(a.borrow_ref());
                let b = vm_try!(b.borrow_ref());

                if a.rtti().enum_hash == b.rtti().enum_hash {
                    return Variant::eq_with(&a, &b, caller);
                }
            }
            (Self::String(a), Self::String(b)) => {
                return VmResult::Ok(*vm_try!(a.borrow_ref()) == *vm_try!(b.borrow_ref()));
            }
            (Self::Option(a), Self::Option(b)) => {
                match (&*vm_try!(a.borrow_ref()), &*vm_try!(b.borrow_ref())) {
                    (Some(a), Some(b)) => return Self::eq_with(a, b, caller),
                    (None, None) => return VmResult::Ok(true),
                    _ => return VmResult::Ok(false),
                }
            }
            (Self::Result(a), Self::Result(b)) => {
                match (&*vm_try!(a.borrow_ref()), &*vm_try!(b.borrow_ref())) {
                    (Ok(a), Ok(b)) => return Self::eq_with(a, b, caller),
                    (Err(a), Err(b)) => return Self::eq_with(a, b, caller),
                    _ => return VmResult::Ok(false),
                }
            }
            _ => {
                match vm_try!(caller.try_call_protocol_fn(Protocol::EQ, self.clone(), (b.clone(),)))
                {
                    CallResult::Ok(value) => return bool::from_value(value),
                    CallResult::Unsupported(..) => {}
                }
            }
        }

        err(VmErrorKind::UnsupportedBinaryOperation {
            op: "eq",
            lhs: vm_try!(self.type_info()),
            rhs: vm_try!(b.type_info()),
        })
    }

    /// Perform a partial ordering comparison between two values.
    ///
    /// This is the basis for the comparison operation.
    ///
    /// External types will use the [`Protocol::PARTIAL_CMP`] protocol when
    /// invoked through this function.
    ///
    /// # Errors
    ///
    /// This function will error if called outside of a virtual machine context.
    pub fn partial_cmp(a: &Value, b: &Value) -> VmResult<Option<Ordering>> {
        Value::partial_cmp_with(a, b, &mut EnvProtocolCaller)
    }

    /// Perform a partial ordering comparison between two values.
    ///
    /// This is the basis for the comparison operation.
    pub(crate) fn partial_cmp_with(
        a: &Value,
        b: &Value,
        caller: &mut impl ProtocolCaller,
    ) -> VmResult<Option<Ordering>> {
        match (a, b) {
            (Self::Bool(a), Self::Bool(b)) => return VmResult::Ok(a.partial_cmp(b)),
            (Self::Byte(a), Self::Byte(b)) => return VmResult::Ok(a.partial_cmp(b)),
            (Self::Char(a), Self::Char(b)) => return VmResult::Ok(a.partial_cmp(b)),
            (Self::Float(a), Self::Float(b)) => return VmResult::Ok(a.partial_cmp(b)),
            (Self::Integer(a), Self::Integer(b)) => return VmResult::Ok(a.partial_cmp(b)),
            (Self::Type(a), Self::Type(b)) => return VmResult::Ok(a.partial_cmp(b)),
            (Self::Bytes(a), Self::Bytes(b)) => {
                let a = vm_try!(a.borrow_ref());
                let b = vm_try!(b.borrow_ref());
                return VmResult::Ok(a.partial_cmp(&b));
            }
            (Self::Vec(a), Self::Vec(b)) => {
                let a = vm_try!(a.borrow_ref());
                let b = vm_try!(b.borrow_ref());
                return Vec::partial_cmp_with(&a, &b, caller);
            }
            (Self::EmptyTuple, Self::EmptyTuple) => return VmResult::Ok(Some(Ordering::Equal)),
            (Self::Tuple(a), Self::Tuple(b)) => {
                let a = vm_try!(a.borrow_ref());
                let b = vm_try!(b.borrow_ref());
                return Vec::partial_cmp_with(&a, &b, caller);
            }
            (Self::Object(a), Self::Object(b)) => {
                let a = vm_try!(a.borrow_ref());
                let b = vm_try!(b.borrow_ref());
                return Object::partial_cmp_with(&a, &b, caller);
            }
            (Self::RangeFrom(a), Self::RangeFrom(b)) => {
                let a = vm_try!(a.borrow_ref());
                let b = vm_try!(b.borrow_ref());
                return RangeFrom::partial_cmp_with(&a, &b, caller);
            }
            (Self::RangeFull(a), Self::RangeFull(b)) => {
                let a = vm_try!(a.borrow_ref());
                let b = vm_try!(b.borrow_ref());
                return RangeFull::partial_cmp_with(&a, &b, caller);
            }
            (Self::RangeInclusive(a), Self::RangeInclusive(b)) => {
                let a = vm_try!(a.borrow_ref());
                let b = vm_try!(b.borrow_ref());
                return RangeInclusive::partial_cmp_with(&a, &b, caller);
            }
            (Self::RangeToInclusive(a), Self::RangeToInclusive(b)) => {
                let a = vm_try!(a.borrow_ref());
                let b = vm_try!(b.borrow_ref());
                return RangeToInclusive::partial_cmp_with(&a, &b, caller);
            }
            (Self::RangeTo(a), Self::RangeTo(b)) => {
                let a = vm_try!(a.borrow_ref());
                let b = vm_try!(b.borrow_ref());
                return RangeTo::partial_cmp_with(&a, &b, caller);
            }
            (Self::Range(a), Self::Range(b)) => {
                let a = vm_try!(a.borrow_ref());
                let b = vm_try!(b.borrow_ref());
                return Range::partial_cmp_with(&a, &b, caller);
            }
            (Self::EmptyStruct(a), Self::EmptyStruct(b)) => {
                if vm_try!(a.borrow_ref()).rtti.hash == vm_try!(b.borrow_ref()).rtti.hash {
                    // NB: don't get any future ideas, this must fall through to
                    // the VmError below since it's otherwise a comparison
                    // between two incompatible types.
                    //
                    // Other than that, all units are equal.
                    return VmResult::Ok(Some(Ordering::Equal));
                }
            }
            (Self::TupleStruct(a), Self::TupleStruct(b)) => {
                let a = vm_try!(a.borrow_ref());
                let b = vm_try!(b.borrow_ref());

                if a.rtti.hash == b.rtti.hash {
                    return Vec::partial_cmp_with(&a.data, &b.data, caller);
                }
            }
            (Self::Struct(a), Self::Struct(b)) => {
                let a = vm_try!(a.borrow_ref());
                let b = vm_try!(b.borrow_ref());

                if a.rtti.hash == b.rtti.hash {
                    return Object::partial_cmp_with(&a.data, &b.data, caller);
                }
            }
            (Self::Variant(a), Self::Variant(b)) => {
                let a = vm_try!(a.borrow_ref());
                let b = vm_try!(b.borrow_ref());

                if a.rtti().enum_hash == b.rtti().enum_hash {
                    return Variant::partial_cmp_with(&a, &b, caller);
                }
            }
            (Self::String(a), Self::String(b)) => {
                let a = vm_try!(a.borrow_ref());
                let b = vm_try!(b.borrow_ref());
                return VmResult::Ok((*a).partial_cmp(&*b));
            }
            (Self::Option(a), Self::Option(b)) => {
                match (&*vm_try!(a.borrow_ref()), &*vm_try!(b.borrow_ref())) {
                    (Some(a), Some(b)) => return Self::partial_cmp_with(a, b, caller),
                    (None, None) => return VmResult::Ok(Some(Ordering::Equal)),
                    (Some(..), None) => return VmResult::Ok(Some(Ordering::Greater)),
                    (None, Some(..)) => return VmResult::Ok(Some(Ordering::Less)),
                }
            }
            (Self::Result(a), Self::Result(b)) => {
                match (&*vm_try!(a.borrow_ref()), &*vm_try!(b.borrow_ref())) {
                    (Ok(a), Ok(b)) => return Self::partial_cmp_with(a, b, caller),
                    (Err(a), Err(b)) => return Self::partial_cmp_with(a, b, caller),
                    (Ok(..), Err(..)) => return VmResult::Ok(Some(Ordering::Greater)),
                    (Err(..), Ok(..)) => return VmResult::Ok(Some(Ordering::Less)),
                }
            }
            (a, b) => {
                match vm_try!(caller.try_call_protocol_fn(
                    Protocol::PARTIAL_CMP,
                    a.clone(),
                    (b.clone(),)
                )) {
                    CallResult::Ok(value) => return <Option<Ordering>>::from_value(value),
                    CallResult::Unsupported(..) => {}
                }
            }
        }

        err(VmErrorKind::UnsupportedBinaryOperation {
            op: "partial_cmp",
            lhs: vm_try!(a.type_info()),
            rhs: vm_try!(b.type_info()),
        })
    }

    /// Perform a total ordering comparison between two values.
    ///
    /// This is the basis for the comparison operation (`cmp`).
    ///
    /// External types will use the [`Protocol::CMP`] protocol when invoked
    /// through this function.
    ///
    /// # Errors
    ///
    /// This function will error if called outside of a virtual machine context.
    pub fn cmp(a: &Value, b: &Value) -> VmResult<Ordering> {
        Value::cmp_with(a, b, &mut EnvProtocolCaller)
    }

    /// Perform a total ordering comparison between two values.
    ///
    /// This is the basis for the comparison operation (`cmp`).
    pub(crate) fn cmp_with(
        a: &Value,
        b: &Value,
        caller: &mut impl ProtocolCaller,
    ) -> VmResult<Ordering> {
        match (a, b) {
            (Self::Bool(a), Self::Bool(b)) => return VmResult::Ok(a.cmp(b)),
            (Self::Byte(a), Self::Byte(b)) => return VmResult::Ok(a.cmp(b)),
            (Self::Char(a), Self::Char(b)) => return VmResult::Ok(a.cmp(b)),
            (Self::Float(a), Self::Float(b)) => {
                if let Some(ordering) = a.partial_cmp(b) {
                    return VmResult::Ok(ordering);
                }

                return VmResult::err(VmErrorKind::IllegalFloatComparison { lhs: *a, rhs: *b });
            }
            (Self::Integer(a), Self::Integer(b)) => return VmResult::Ok(a.cmp(b)),
            (Self::Type(a), Self::Type(b)) => return VmResult::Ok(a.cmp(b)),
            (Self::Bytes(a), Self::Bytes(b)) => {
                let a = vm_try!(a.borrow_ref());
                let b = vm_try!(b.borrow_ref());
                return VmResult::Ok(a.cmp(&b));
            }
            (Self::Vec(a), Self::Vec(b)) => {
                let a = vm_try!(a.borrow_ref());
                let b = vm_try!(b.borrow_ref());
                return Vec::cmp_with(&a, &b, caller);
            }
            (Self::EmptyTuple, Self::EmptyTuple) => return VmResult::Ok(Ordering::Equal),
            (Self::Tuple(a), Self::Tuple(b)) => {
                let a = vm_try!(a.borrow_ref());
                let b = vm_try!(b.borrow_ref());
                return Vec::cmp_with(&a, &b, caller);
            }
            (Self::Object(a), Self::Object(b)) => {
                let a = vm_try!(a.borrow_ref());
                let b = vm_try!(b.borrow_ref());
                return Object::cmp_with(&a, &b, caller);
            }
            (Self::RangeFrom(a), Self::RangeFrom(b)) => {
                let a = vm_try!(a.borrow_ref());
                let b = vm_try!(b.borrow_ref());
                return RangeFrom::cmp_with(&a, &b, caller);
            }
            (Self::RangeFull(a), Self::RangeFull(b)) => {
                let a = vm_try!(a.borrow_ref());
                let b = vm_try!(b.borrow_ref());
                return RangeFull::cmp_with(&a, &b, caller);
            }
            (Self::RangeInclusive(a), Self::RangeInclusive(b)) => {
                let a = vm_try!(a.borrow_ref());
                let b = vm_try!(b.borrow_ref());
                return RangeInclusive::cmp_with(&a, &b, caller);
            }
            (Self::RangeToInclusive(a), Self::RangeToInclusive(b)) => {
                let a = vm_try!(a.borrow_ref());
                let b = vm_try!(b.borrow_ref());
                return RangeToInclusive::cmp_with(&a, &b, caller);
            }
            (Self::RangeTo(a), Self::RangeTo(b)) => {
                let a = vm_try!(a.borrow_ref());
                let b = vm_try!(b.borrow_ref());
                return RangeTo::cmp_with(&a, &b, caller);
            }
            (Self::Range(a), Self::Range(b)) => {
                let a = vm_try!(a.borrow_ref());
                let b = vm_try!(b.borrow_ref());
                return Range::cmp_with(&a, &b, caller);
            }
            (Self::EmptyStruct(a), Self::EmptyStruct(b)) => {
                if vm_try!(a.borrow_ref()).rtti.hash == vm_try!(b.borrow_ref()).rtti.hash {
                    // NB: don't get any future ideas, this must fall through to
                    // the VmError below since it's otherwise a comparison
                    // between two incompatible types.
                    //
                    // Other than that, all units are equal.
                    return VmResult::Ok(Ordering::Equal);
                }
            }
            (Self::TupleStruct(a), Self::TupleStruct(b)) => {
                let a = vm_try!(a.borrow_ref());
                let b = vm_try!(b.borrow_ref());

                if a.rtti.hash == b.rtti.hash {
                    return Vec::cmp_with(&a.data, &b.data, caller);
                }
            }
            (Self::Struct(a), Self::Struct(b)) => {
                let a = vm_try!(a.borrow_ref());
                let b = vm_try!(b.borrow_ref());

                if a.rtti.hash == b.rtti.hash {
                    return Object::cmp_with(&a.data, &b.data, caller);
                }
            }
            (Self::Variant(a), Self::Variant(b)) => {
                let a = vm_try!(a.borrow_ref());
                let b = vm_try!(b.borrow_ref());

                if a.rtti().enum_hash == b.rtti().enum_hash {
                    return Variant::cmp_with(&a, &b, caller);
                }
            }
            (Self::String(a), Self::String(b)) => {
                let a = vm_try!(a.borrow_ref());
                let b = vm_try!(b.borrow_ref());
                return VmResult::Ok(a.cmp(&b));
            }
            (Self::Option(a), Self::Option(b)) => {
                match (&*vm_try!(a.borrow_ref()), &*vm_try!(b.borrow_ref())) {
                    (Some(a), Some(b)) => return Self::cmp_with(a, b, caller),
                    (None, None) => return VmResult::Ok(Ordering::Equal),
                    (Some(..), None) => return VmResult::Ok(Ordering::Greater),
                    (None, Some(..)) => return VmResult::Ok(Ordering::Less),
                }
            }
            (Self::Result(a), Self::Result(b)) => {
                match (&*vm_try!(a.borrow_ref()), &*vm_try!(b.borrow_ref())) {
                    (Ok(a), Ok(b)) => return Self::cmp_with(a, b, caller),
                    (Err(a), Err(b)) => return Self::cmp_with(a, b, caller),
                    (Ok(..), Err(..)) => return VmResult::Ok(Ordering::Greater),
                    (Err(..), Ok(..)) => return VmResult::Ok(Ordering::Less),
                }
            }
            (a, b) => {
                match vm_try!(caller.try_call_protocol_fn(Protocol::CMP, a.clone(), (b.clone(),))) {
                    CallResult::Ok(value) => return Ordering::from_value(value),
                    CallResult::Unsupported(..) => {}
                }
            }
        }

        err(VmErrorKind::UnsupportedBinaryOperation {
            op: "cmp",
            lhs: vm_try!(a.type_info()),
            rhs: vm_try!(b.type_info()),
        })
    }

    pub(crate) fn try_into_integer<T>(self) -> VmResult<T>
    where
        T: TryFrom<i64>,
        VmIntegerRepr: From<i64>,
    {
        let integer = vm_try!(self.into_integer());

        match integer.try_into() {
            Ok(number) => VmResult::Ok(number),
            Err(..) => VmResult::err(VmErrorKind::ValueToIntegerCoercionError {
                from: VmIntegerRepr::from(integer),
                to: any::type_name::<T>(),
            }),
        }
    }

    pub(crate) fn try_as_integer<T>(&self) -> VmResult<T>
    where
        T: TryFrom<i64>,
        VmIntegerRepr: From<i64>,
    {
        let integer = vm_try!(self.as_integer());

        match integer.try_into() {
            Ok(number) => VmResult::Ok(number),
            Err(..) => VmResult::err(VmErrorKind::ValueToIntegerCoercionError {
                from: VmIntegerRepr::from(integer),
                to: any::type_name::<T>(),
            }),
        }
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
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
            Value::String(value) => {
                write!(f, "{:?}", value)?;
            }
            Value::Bytes(value) => {
                write!(f, "{:?}", value)?;
            }
            Value::Vec(value) => {
                write!(f, "{:?}", value)?;
            }
            Value::EmptyTuple => {
                write!(f, "()")?;
            }
            Value::Tuple(value) => {
                write!(f, "{:?}", value)?;
            }
            Value::Object(value) => {
                write!(f, "{:?}", value)?;
            }
            Value::RangeFrom(value) => {
                write!(f, "{:?}", value)?;
            }
            Value::RangeFull(value) => {
                write!(f, "{:?}", value)?;
            }
            Value::RangeInclusive(value) => {
                write!(f, "{:?}", value)?;
            }
            Value::RangeToInclusive(value) => {
                write!(f, "{:?}", value)?;
            }
            Value::RangeTo(value) => {
                write!(f, "{:?}", value)?;
            }
            Value::Range(value) => {
                write!(f, "{:?}", value)?;
            }
            Value::ControlFlow(value) => {
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
            Value::EmptyStruct(value) => {
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
                let mut o = Formatter::new();

                if value.string_debug(&mut o).is_err() {
                    return Err(fmt::Error);
                }

                f.write_str(o.as_str())?;
            }
        }

        Ok(())
    }
}

impl Default for Value {
    fn default() -> Self {
        Self::EmptyTuple
    }
}

impl From<()> for Value {
    fn from((): ()) -> Self {
        Self::EmptyTuple
    }
}

impl ToValue for Value {
    fn to_value(self) -> VmResult<Value> {
        VmResult::Ok(self)
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
                    VmResult::Ok(Value::from(self))
                }
            }
        )*
    };
}

macro_rules! impl_from_wrapper {
    ($($variant:ident => $wrapper:ident<$ty:ty>),* $(,)?) => {
        impl_from!($($variant => $wrapper<$ty>),*);

        $(
            impl TryFrom<$ty> for Value {
                type Error = rune_alloc::Error;

                #[inline]
                fn try_from(value: $ty) -> Result<Self, rune_alloc::Error> {
                    Ok(Self::$variant($wrapper::new(value)?))
                }
            }

            impl ToValue for $ty {
                #[inline]
                fn to_value(self) -> VmResult<Value> {
                    VmResult::Ok(vm_try!(Value::try_from(self)))
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
    Type => Type,
    Option => Shared<Option<Value>>,
    Result => Shared<Result<Value, Value>>,
}

impl_from_wrapper! {
    Format => Shared<Format>,
    Iterator => Shared<Iterator>,
    Bytes => Shared<Bytes>,
    String => Shared<String>,
    Vec => Shared<Vec>,
    Tuple => Shared<OwnedTuple>,
    Object => Shared<Object>,
    RangeFrom => Shared<RangeFrom>,
    RangeFull => Shared<RangeFull>,
    RangeInclusive => Shared<RangeInclusive>,
    RangeToInclusive => Shared<RangeToInclusive>,
    RangeTo => Shared<RangeTo>,
    Range => Shared<Range>,
    ControlFlow => Shared<ControlFlow>,
    Future => Shared<Future>,
    Stream => Shared<Stream<Vm>>,
    Generator => Shared<Generator<Vm>>,
    GeneratorState => Shared<GeneratorState>,
    EmptyStruct => Shared<EmptyStruct>,
    TupleStruct => Shared<TupleStruct>,
    Struct => Shared<Struct>,
    Variant => Shared<Variant>,
    Function => Shared<Function>,
    Any => Shared<AnyObj>,
}

impl MaybeTypeOf for Value {
    #[inline]
    fn maybe_type_of() -> Option<FullTypeOf> {
        None
    }
}

impl TryClone for Value {
    fn try_clone(&self) -> alloc::Result<Self> {
        // NB: value cloning is a shallow clone of the underlying data.
        Ok(self.clone())
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
