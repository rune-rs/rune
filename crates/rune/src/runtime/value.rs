#[macro_use]
mod macros;

mod serde;

mod rtti;
pub use self::rtti::{Rtti, VariantRtti};

mod data;
pub use self::data::{EmptyStruct, Struct, TupleStruct};

use core::any;
use core::cmp::{Ord, Ordering, PartialOrd};
use core::fmt;

use ::rust_alloc::sync::Arc;

use crate::alloc::fmt::TryWrite;
use crate::alloc::prelude::*;
use crate::alloc::{self, String};
use crate::compile::meta;
use crate::runtime::static_type;
use crate::runtime::vm::CallResultOnly;
use crate::runtime::{
    AccessError, AccessErrorKind, AnyObj, AnyObjError, BorrowMut, BorrowRef, Bytes, ConstValue,
    ControlFlow, DynGuardedArgs, EnvProtocolCaller, Format, Formatter, FromValue, Function, Future,
    Generator, GeneratorState, IntoOutput, Iterator, MaybeTypeOf, Mut, Object, OwnedTuple,
    Protocol, ProtocolCaller, Range, RangeFrom, RangeFull, RangeInclusive, RangeTo,
    RangeToInclusive, Ref, RuntimeError, Shared, SharedPointerGuard, Snapshot, Stream, ToValue,
    Type, TypeInfo, Variant, Vec, Vm, VmErrorKind, VmIntegerRepr, VmResult,
};
#[cfg(feature = "alloc")]
use crate::runtime::{Hasher, Tuple};
use crate::{Any, Hash};

// Small helper function to build errors.
fn err<T, E>(error: E) -> VmResult<T>
where
    VmErrorKind: From<E>,
{
    VmResult::err(error)
}

#[derive(Clone)]
enum Repr {
    Empty,
    Inline(Inline),
    Mutable(Shared<Mutable>),
}

pub(crate) enum ValueRepr {
    Inline(Inline),
    Mutable(Shared<Mutable>),
}

pub(crate) enum OwnedValue {
    Inline(Inline),
    Mutable(Mutable),
}

impl OwnedValue {
    #[inline]
    pub(crate) fn type_info(&self) -> TypeInfo {
        match self {
            OwnedValue::Inline(value) => value.type_info(),
            OwnedValue::Mutable(value) => value.type_info(),
        }
    }
}

pub(crate) enum ValueRef<'a> {
    Inline(&'a Inline),
    Mutable(&'a Shared<Mutable>),
}

impl ValueRef<'_> {
    #[inline]
    pub(crate) fn type_info(&self) -> Result<TypeInfo, AccessError> {
        match self {
            ValueRef::Inline(value) => Ok(value.type_info()),
            ValueRef::Mutable(value) => Ok(value.borrow_ref()?.type_info()),
        }
    }
}

pub(crate) enum ValueBorrowRef<'a> {
    Inline(&'a Inline),
    Mutable(BorrowRef<'a, Mutable>),
}

impl<'a> ValueBorrowRef<'a> {
    #[inline]
    pub(crate) fn type_info(&self) -> TypeInfo {
        match self {
            ValueBorrowRef::Inline(value) => value.type_info(),
            ValueBorrowRef::Mutable(value) => value.type_info(),
        }
    }
}

/// Access the internals of a value mutably.
pub(crate) enum ValueMut<'a> {
    Inline(&'a mut Inline),
    Mutable(#[allow(unused)] &'a mut Shared<Mutable>),
}

pub(crate) enum ValueShared {
    Inline(Inline),
    Mutable(Shared<Mutable>),
}

/// An entry on the stack.
pub struct Value {
    repr: Repr,
}

impl Value {
    /// Construct an Any that wraps a pointer.
    ///
    /// # Safety
    ///
    /// Caller must ensure that the returned `Value` doesn't outlive the
    /// reference it is wrapping.
    ///
    /// This would be an example of incorrect use:
    ///
    /// ```no_run
    /// use rune::Any;
    /// use rune::runtime::Value;
    ///
    /// #[derive(Any)]
    /// struct Foo(u32);
    ///
    /// let mut v = Foo(1u32);
    ///
    /// unsafe {
    ///     let (any, guard) = unsafe { Value::from_ref(&v)? };
    ///     drop(v);
    ///     // any use of `any` beyond here is undefined behavior.
    /// }
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::Any;
    /// use rune::runtime::Value;
    ///
    /// #[derive(Any)]
    /// struct Foo(u32);
    ///
    /// let mut v = Foo(1u32);
    ///
    /// unsafe {
    ///     let (any, guard) = Value::from_ref(&mut v)?;
    ///     let b = any.into_any_ref::<Foo>().unwrap();
    ///     assert_eq!(b.0, 1u32);
    /// }
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    pub unsafe fn from_ref<T>(data: &T) -> alloc::Result<(Self, SharedPointerGuard)>
    where
        T: Any,
    {
        let value = Shared::new(Mutable::Any(AnyObj::from_ref(data)))?;
        let (value, guard) = Shared::into_drop_guard(value);
        Ok((
            Self {
                repr: Repr::Mutable(value),
            },
            guard,
        ))
    }

    /// Optionally get the snapshot of the value if available.
    pub(crate) fn snapshot(&self) -> Option<Snapshot> {
        match &self.repr {
            Repr::Mutable(value) => Some(value.snapshot()),
            _ => None,
        }
    }

    /// Construct a value that wraps a mutable pointer.
    ///
    /// # Safety
    ///
    /// Caller must ensure that the returned `Value` doesn't outlive the
    /// reference it is wrapping.
    ///
    /// This would be an example of incorrect use:
    ///
    /// ```no_run
    /// use rune::Any;
    /// use rune::runtime::Value;
    ///
    /// #[derive(Any)]
    /// struct Foo(u32);
    ///
    /// let mut v = Foo(1u32);
    /// unsafe {
    ///     let (any, guard) = Value::from_mut(&mut v)?;
    ///     drop(v);
    ///     // any use of value beyond here is undefined behavior.
    /// }
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::Any;
    /// use rune::runtime::{Value, VmResult};
    ///
    /// #[derive(Any)]
    /// struct Foo(u32);
    ///
    /// let mut v = Foo(1u32);
    ///
    /// unsafe {
    ///     let (mut any, guard) = Value::from_mut(&mut v)?;
    ///
    ///     if let Ok(mut v) = any.into_any_mut::<Foo>() {
    ///         v.0 += 1;
    ///     }
    /// }
    ///
    /// assert_eq!(v.0, 2);
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    pub unsafe fn from_mut<T>(data: &mut T) -> alloc::Result<(Self, SharedPointerGuard)>
    where
        T: Any,
    {
        let obj = AnyObj::from_mut(data);
        let value = Shared::new(Mutable::Any(obj))?;
        let (value, guard) = Shared::into_drop_guard(value);
        Ok((
            Self {
                repr: Repr::Mutable(value),
            },
            guard,
        ))
    }

    /// Test if the value is writable.
    pub fn is_writable(&self) -> bool {
        match self.repr {
            Repr::Empty => false,
            Repr::Inline(..) => true,
            Repr::Mutable(ref value) => value.is_writable(),
        }
    }

    /// Test if the value is readable.
    pub fn is_readable(&self) -> bool {
        match &self.repr {
            Repr::Empty => false,
            Repr::Inline(..) => true,
            Repr::Mutable(ref value) => value.is_readable(),
        }
    }

    /// Construct a unit value.
    pub(crate) const fn unit() -> Self {
        Self {
            repr: Repr::Inline(Inline::Unit),
        }
    }

    /// Construct an empty value.
    pub const fn empty() -> Self {
        Self { repr: Repr::Empty }
    }

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
    #[cfg_attr(feature = "bench", inline(never))]
    pub(crate) fn string_display_with(
        &self,
        f: &mut Formatter,
        caller: &mut dyn ProtocolCaller,
    ) -> VmResult<()> {
        'fallback: {
            match vm_try!(self.borrow_ref()) {
                ValueBorrowRef::Inline(value) => match value {
                    Inline::Char(c) => {
                        vm_try!(f.push(*c));
                    }
                    Inline::Integer(integer) => {
                        let mut buffer = itoa::Buffer::new();
                        vm_try!(f.push_str(buffer.format(*integer)));
                    }
                    Inline::Float(float) => {
                        let mut buffer = ryu::Buffer::new();
                        vm_try!(f.push_str(buffer.format(*float)));
                    }
                    Inline::Bool(bool) => {
                        vm_write!(f, "{bool}");
                    }
                    Inline::Byte(byte) => {
                        let mut buffer = itoa::Buffer::new();
                        vm_try!(f.push_str(buffer.format(*byte)));
                    }
                    _ => {
                        break 'fallback;
                    }
                },
                ValueBorrowRef::Mutable(value) => match &*value {
                    Mutable::Format(format) => {
                        vm_try!(format.spec.format(&format.value, f, caller));
                    }
                    Mutable::String(string) => {
                        vm_try!(f.push_str(string));
                    }
                    _ => {
                        break 'fallback;
                    }
                },
            }

            return VmResult::Ok(());
        };

        let mut args = DynGuardedArgs::new((f,));

        let result =
            vm_try!(caller.call_protocol_fn(Protocol::STRING_DISPLAY, self.clone(), &mut args));

        <()>::from_value(result)
    }

    /// Perform a shallow clone of the value using the [`CLONE`] protocol.
    ///
    /// This requires read access to the underlying value.
    ///
    /// You must use [Vm::with] to specify which virtual machine this function
    /// is called inside.
    ///
    /// # Panics
    ///
    /// This function will panic if called outside of a virtual machine.
    ///
    /// [`CLONE`]: Protocol::CLONE
    pub fn clone_(&self) -> VmResult<Self> {
        self.clone_with(&mut EnvProtocolCaller)
    }

    pub(crate) fn clone_with(&self, caller: &mut dyn ProtocolCaller) -> VmResult<Value> {
        'fallback: {
            let value = match vm_try!(self.borrow_ref()) {
                ValueBorrowRef::Inline(value) => {
                    return VmResult::Ok(Self {
                        repr: Repr::Inline(*value),
                    });
                }
                ValueBorrowRef::Mutable(value) => match &*value {
                    Mutable::String(value) => Mutable::String(vm_try!(value.try_clone())),
                    Mutable::Bytes(value) => Mutable::Bytes(vm_try!(value.try_clone())),
                    Mutable::Vec(value) => Mutable::Vec(vm_try!(value.try_clone())),
                    Mutable::Tuple(value) => Mutable::Tuple(vm_try!(value.try_clone())),
                    Mutable::Object(value) => Mutable::Object(vm_try!(value.try_clone())),
                    Mutable::RangeFrom(value) => Mutable::RangeFrom(vm_try!(value.try_clone())),
                    Mutable::RangeFull(value) => Mutable::RangeFull(vm_try!(value.try_clone())),
                    Mutable::RangeInclusive(value) => {
                        Mutable::RangeInclusive(vm_try!(value.try_clone()))
                    }
                    Mutable::RangeToInclusive(value) => {
                        Mutable::RangeToInclusive(vm_try!(value.try_clone()))
                    }
                    Mutable::RangeTo(value) => Mutable::RangeTo(vm_try!(value.try_clone())),
                    Mutable::Range(value) => Mutable::Range(vm_try!(value.try_clone())),
                    Mutable::ControlFlow(value) => Mutable::ControlFlow(vm_try!(value.try_clone())),
                    Mutable::Stream(value) => Mutable::Stream(vm_try!(value.try_clone())),
                    Mutable::Generator(value) => Mutable::Generator(vm_try!(value.try_clone())),
                    Mutable::GeneratorState(value) => {
                        Mutable::GeneratorState(vm_try!(value.try_clone()))
                    }
                    Mutable::Option(value) => Mutable::Option(vm_try!(value.try_clone())),
                    Mutable::Result(value) => Mutable::Result(vm_try!(value.try_clone())),
                    Mutable::EmptyStruct(value) => Mutable::EmptyStruct(vm_try!(value.try_clone())),
                    Mutable::TupleStruct(value) => Mutable::TupleStruct(vm_try!(value.try_clone())),
                    Mutable::Struct(value) => Mutable::Struct(vm_try!(value.try_clone())),
                    Mutable::Variant(value) => Mutable::Variant(vm_try!(value.try_clone())),
                    Mutable::Function(value) => Mutable::Function(vm_try!(value.try_clone())),
                    Mutable::Format(value) => Mutable::Format(vm_try!(value.try_clone())),
                    _ => {
                        break 'fallback;
                    }
                },
            };

            return VmResult::Ok(Self {
                repr: Repr::Mutable(vm_try!(Shared::new(value))),
            });
        };

        VmResult::Ok(vm_try!(caller.call_protocol_fn(
            Protocol::CLONE,
            self.clone(),
            &mut ()
        )))
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
        caller: &mut dyn ProtocolCaller,
    ) -> VmResult<()> {
        'fallback: {
            let value = match self.repr {
                Repr::Empty => {
                    vm_write!(f, "<empty>");
                    return VmResult::Ok(());
                }
                Repr::Inline(value) => {
                    vm_write!(f, "{value:?}");
                    return VmResult::Ok(());
                }
                Repr::Mutable(ref value) => value,
            };

            match &*vm_try!(value.borrow_ref()) {
                Mutable::String(value) => {
                    vm_write!(f, "{:?}", value);
                }
                Mutable::Bytes(value) => {
                    vm_write!(f, "{:?}", value);
                }
                Mutable::Vec(value) => {
                    vm_try!(Vec::string_debug_with(value, f, caller));
                }
                Mutable::Tuple(value) => {
                    vm_write!(f, "{:?}", value);
                }
                Mutable::Object(value) => {
                    vm_write!(f, "{:?}", value);
                }
                Mutable::RangeFrom(value) => {
                    vm_write!(f, "{:?}", value);
                }
                Mutable::RangeFull(value) => {
                    vm_write!(f, "{:?}", value);
                }
                Mutable::RangeInclusive(value) => {
                    vm_write!(f, "{:?}", value);
                }
                Mutable::RangeToInclusive(value) => {
                    vm_write!(f, "{:?}", value);
                }
                Mutable::RangeTo(value) => {
                    vm_write!(f, "{:?}", value);
                }
                Mutable::Range(value) => {
                    vm_write!(f, "{:?}", value);
                }
                Mutable::ControlFlow(value) => {
                    vm_try!(ControlFlow::string_debug_with(value, f, caller));
                }
                Mutable::Future(value) => {
                    vm_write!(f, "{:?}", value);
                }
                Mutable::Stream(value) => {
                    vm_write!(f, "{:?}", value);
                }
                Mutable::Generator(value) => {
                    vm_write!(f, "{:?}", value);
                }
                Mutable::GeneratorState(value) => {
                    vm_write!(f, "{:?}", value);
                }
                Mutable::Option(value) => {
                    vm_write!(f, "{:?}", value);
                }
                Mutable::Result(value) => {
                    vm_write!(f, "{:?}", value);
                }
                Mutable::EmptyStruct(value) => {
                    vm_write!(f, "{:?}", value);
                }
                Mutable::TupleStruct(value) => {
                    vm_write!(f, "{:?}", value);
                }
                Mutable::Struct(value) => {
                    vm_write!(f, "{:?}", value);
                }
                Mutable::Variant(value) => {
                    vm_write!(f, "{:?}", value);
                }
                Mutable::Function(value) => {
                    vm_write!(f, "{:?}", value);
                }
                Mutable::Format(value) => {
                    vm_write!(f, "{:?}", value);
                }
                _ => {
                    break 'fallback;
                }
            };

            return VmResult::Ok(());
        };

        // reborrow f to avoid moving it
        let mut args = DynGuardedArgs::new((&mut *f,));

        match vm_try!(caller.try_call_protocol_fn(Protocol::STRING_DEBUG, self.clone(), &mut args))
        {
            CallResultOnly::Ok(value) => {
                vm_try!(<()>::from_value(value));
            }
            CallResultOnly::Unsupported(value) => match &value.repr {
                Repr::Empty => {
                    vm_write!(f, "<empty>");
                }
                Repr::Inline(value) => {
                    vm_write!(f, "{value:?}");
                }
                Repr::Mutable(value) => {
                    let ty = vm_try!(value.borrow_ref()).type_info();
                    vm_write!(f, "<{ty} object at {value:p}>");
                }
            },
        }

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

    pub(crate) fn into_iter_with(self, caller: &mut dyn ProtocolCaller) -> VmResult<Iterator> {
        let value = vm_try!(caller.call_protocol_fn(Protocol::INTO_ITER, self, &mut ()));
        VmResult::Ok(Iterator::new(value))
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

        crate::runtime::env::shared(|context, unit| {
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
        let data = Vec::from(vec);

        VmResult::Ok(vm_try!(Value::try_from(data)))
    }

    /// Construct a tuple.
    pub fn tuple(vec: alloc::Vec<Value>) -> VmResult<Self> {
        let data = vm_try!(OwnedTuple::try_from(vec));

        VmResult::Ok(vm_try!(Value::try_from(data)))
    }

    /// Construct an empty.
    pub fn empty_struct(rtti: Arc<Rtti>) -> VmResult<Self> {
        VmResult::Ok(vm_try!(Value::try_from(EmptyStruct { rtti })))
    }

    /// Construct a typed tuple.
    pub fn tuple_struct(rtti: Arc<Rtti>, vec: alloc::Vec<Value>) -> VmResult<Self> {
        let data = vm_try!(OwnedTuple::try_from(vec));
        VmResult::Ok(vm_try!(Value::try_from(TupleStruct { rtti, data })))
    }

    /// Construct an empty variant.
    pub fn unit_variant(rtti: Arc<VariantRtti>) -> VmResult<Self> {
        VmResult::Ok(vm_try!(Value::try_from(Variant::unit(rtti))))
    }

    /// Construct a tuple variant.
    pub fn tuple_variant(rtti: Arc<VariantRtti>, vec: alloc::Vec<Value>) -> VmResult<Self> {
        let data = vm_try!(OwnedTuple::try_from(vec));

        VmResult::Ok(vm_try!(Value::try_from(Variant::tuple(rtti, data))))
    }

    /// Drop the interior value.
    pub(crate) fn drop(self) -> VmResult<()> {
        if let Repr::Mutable(value) = self.repr {
            drop(vm_try!(value.take()));
        }

        VmResult::Ok(())
    }

    /// Move the interior value.
    pub(crate) fn move_(self) -> VmResult<Self> {
        match self.repr {
            Repr::Mutable(value) => VmResult::Ok(Value {
                repr: Repr::Mutable(vm_try!(Shared::new(vm_try!(value.take())))),
            }),
            repr => VmResult::Ok(Value { repr }),
        }
    }

    /// Try to coerce value into a usize.
    #[inline]
    pub fn as_usize(&self) -> Result<usize, RuntimeError> {
        self.try_as_integer()
    }

    /// Get the value as a string.
    #[deprecated(
        note = "For consistency with other methods, this has been renamed Value::borrow_string_ref"
    )]
    #[inline]
    pub fn as_string(&self) -> Result<BorrowRef<'_, str>, RuntimeError> {
        self.borrow_string_ref()
    }

    /// Borrow the value of a string as a reference.
    pub fn borrow_string_ref(&self) -> Result<BorrowRef<'_, str>, RuntimeError> {
        let value = match self.borrow_ref()? {
            ValueBorrowRef::Mutable(value) => value,
            actual => {
                return Err(RuntimeError::expected::<str>(actual.type_info()));
            }
        };

        let result = BorrowRef::try_map(value, |kind| match kind {
            Mutable::String(string) => Some(string.as_str()),
            _ => None,
        });

        match result {
            Ok(s) => Ok(s),
            Err(actual) => Err(RuntimeError::expected::<str>(actual.type_info())),
        }
    }

    /// Take the current value as a string.
    #[inline]
    pub fn into_string(self) -> Result<String, RuntimeError> {
        match self.take_value()? {
            OwnedValue::Mutable(Mutable::String(string)) => Ok(string),
            actual => Err(RuntimeError::expected::<String>(actual.type_info())),
        }
    }

    /// Coerce into type value.
    #[doc(hidden)]
    #[inline]
    pub fn into_type_value(self) -> Result<TypeValue, RuntimeError> {
        match self.take_value()? {
            OwnedValue::Inline(value) => match value {
                Inline::Unit => Ok(TypeValue::Unit),
                actual => Ok(TypeValue::NotTypedInline(NotTypedInlineValue(actual))),
            },
            OwnedValue::Mutable(value) => match value {
                Mutable::Tuple(tuple) => Ok(TypeValue::Tuple(tuple)),
                Mutable::Object(object) => Ok(TypeValue::Object(object)),
                Mutable::EmptyStruct(empty) => Ok(TypeValue::EmptyStruct(empty)),
                Mutable::TupleStruct(tuple) => Ok(TypeValue::TupleStruct(tuple)),
                Mutable::Struct(object) => Ok(TypeValue::Struct(object)),
                Mutable::Variant(object) => Ok(TypeValue::Variant(object)),
                actual => Ok(TypeValue::NotTypedMutable(NotTypedMutableValue(actual))),
            },
        }
    }

    /// Coerce into a unit.
    #[inline]
    pub fn into_unit(&self) -> Result<(), RuntimeError> {
        match self.borrow_ref()? {
            ValueBorrowRef::Inline(Inline::Unit) => Ok(()),
            ValueBorrowRef::Inline(actual) => Err(RuntimeError::expected::<()>(actual.type_info())),
            ValueBorrowRef::Mutable(actual) => {
                Err(RuntimeError::expected::<()>(actual.type_info()))
            }
        }
    }

    inline_into! {
        /// Coerce into [`Ordering`].
        Ordering(Ordering),
        as_ordering,
        as_ordering_mut,
    }

    inline_into! {
        /// Coerce into [`bool`].
        Bool(bool),
        as_bool,
        as_bool_mut,
    }

    inline_into! {
        /// Coerce into [`u8`] byte.
        Byte(u8),
        as_byte,
        as_byte_mut,
    }

    inline_into! {
        /// Coerce into [`char`].
        Char(char),
        as_char,
        as_char_mut,
    }

    inline_into! {
        /// Coerce into [`i64`] integer.
        Integer(i64),
        as_integer,
        as_integer_mut,
    }

    inline_into! {
        /// Coerce into [`f64`] float.
        Float(f64),
        as_float,
        as_float_mut,
    }

    inline_into! {
        /// Coerce into [`Type`].
        Type(Type),
        as_type,
        as_type_mut,
    }

    clone_into! {
        /// Coerce into [`Option`].
        Option(Option<Value>),
        into_option_ref,
        into_option_mut,
        borrow_option_ref,
        borrow_option_mut,
        as_option,
    }

    clone_into! {
        /// Coerce into [`Result`].
        Result(Result<Value, Value>),
        into_result_ref,
        into_result_mut,
        borrow_result_ref,
        borrow_result_mut,
        as_result,
    }

    into! {
        /// Coerce into [`Vec`].
        Vec(Vec),
        into_vec_ref,
        into_vec_mut,
        borrow_vec_ref,
        borrow_vec_mut,
        into_vec,
    }

    into! {
        /// Coerce into [`Bytes`].
        Bytes(Bytes),
        into_bytes_ref,
        into_bytes_mut,
        borrow_bytes_ref,
        borrow_bytes_mut,
        into_bytes,
    }

    into! {
        /// Coerce into a [`ControlFlow`].
        ControlFlow(ControlFlow),
        into_control_flow_ref,
        into_control_flow_mut,
        borrow_control_flow_ref,
        borrow_control_flow_mut,
        into_control_flow,
    }

    into! {
        /// Coerce into a [`Function`].
        Function(Function),
        into_function_ref,
        into_function_mut,
        borrow_function_ref,
        borrow_function_mut,
        into_function,
    }

    into! {
        /// Coerce into a [`GeneratorState`].
        GeneratorState(GeneratorState),
        into_generator_state_ref,
        into_generator_state_mut,
        borrow_generator_state_ref,
        borrow_generator_state_mut,
        into_generator_state,
    }

    into! {
        /// Coerce into a [`Generator`].
        Generator(Generator<Vm>),
        into_generator_ref,
        into_generator_mut,
        borrow_generator_ref,
        borrow_generator_mut,
        into_generator,
    }

    into! {
        /// Coerce into a [`Format`].
        Format(Format),
        into_format_ref,
        into_format_mut,
        borrow_format_ref,
        borrow_format_mut,
        into_format,
    }

    into! {
        /// Coerce into [`Tuple`].
        Tuple(OwnedTuple),
        into_tuple_ref,
        into_tuple_mut,
        borrow_tuple_ref,
        borrow_tuple_mut,
        into_tuple,
    }

    into! {
        /// Coerce into [`Struct`]
        Struct(Struct),
        into_struct_ref,
        into_struct_mut,
        borrow_struct_ref,
        borrow_struct_mut,
        into_struct,
    }

    into! {
        /// Coerce into a [`Object`].
        Object(Object),
        into_object_ref,
        into_object_mut,
        borrow_object_ref,
        borrow_object_mut,
        into_object,
    }

    into! {
        /// Coerce into a [`RangeFrom`].
        RangeFrom(RangeFrom),
        into_range_from_ref,
        into_range_from_mut,
        borrow_range_from_ref,
        borrow_range_from_mut,
        into_range_from,
    }

    into! {
        /// Coerce into a [`RangeFull`].
        RangeFull(RangeFull),
        into_range_full_ref,
        into_range_full_mut,
        borrow_range_full_ref,
        borrow_range_full_mut,
        into_range_full,
    }

    into! {
        /// Coerce into a [`RangeToInclusive`].
        RangeToInclusive(RangeToInclusive),
        into_range_to_inclusive_ref,
        into_range_to_inclusive_mut,
        borrow_range_to_inclusive_ref,
        borrow_range_to_inclusive_mut,
        into_range_to_inclusive,
    }

    into! {
        /// Coerce into a [`RangeInclusive`].
        RangeInclusive(RangeInclusive),
        into_range_inclusive_ref,
        into_range_inclusive_mut,
        borrow_range_inclusive_ref,
        borrow_range_inclusive_mut,
        into_range_inclusive,
    }

    into! {
        /// Coerce into a [`RangeTo`].
        RangeTo(RangeTo),
        into_range_to_ref,
        into_range_to_mut,
        borrow_range_to_ref,
        borrow_range_to_mut,
        into_range_to,
    }

    into! {
        /// Coerce into a [`Range`].
        Range(Range),
        into_range_ref,
        into_range_mut,
        borrow_range_ref,
        borrow_range_mut,
        into_range,
    }

    into! {
        /// Coerce into a [`Stream`].
        Stream(Stream<Vm>),
        into_stream_ref,
        into_stream_mut,
        borrow_stream_ref,
        borrow_stream_mut,
        into_stream,
    }

    into_base! {
        /// Coerce into a [`Future`].
        Future(Future),
        into_future_ref,
        into_future_mut,
        borrow_future_ref,
        borrow_future_mut,
    }

    /// Coerce into an [`AnyObj`].
    ///
    /// This consumes the underlying value.
    #[inline]
    pub fn into_any_obj(self) -> Result<AnyObj, RuntimeError> {
        match self.take_value()? {
            OwnedValue::Mutable(Mutable::Any(value)) => Ok(value),
            ref actual => Err(RuntimeError::expected_any(actual.type_info())),
        }
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
    pub fn into_future(self) -> VmResult<Future> {
        let target = match vm_try!(self.take_value()) {
            OwnedValue::Mutable(Mutable::Future(future)) => return VmResult::Ok(future),
            OwnedValue::Inline(value) => Value::from(value),
            OwnedValue::Mutable(value) => vm_try!(Value::try_from(value)),
        };

        let value =
            vm_try!(EnvProtocolCaller.call_protocol_fn(Protocol::INTO_FUTURE, target, &mut ()));

        VmResult::Ok(vm_try!(Future::from_value(value)))
    }

    /// Try to coerce value into a typed reference.
    #[inline]
    pub fn into_any_ref<T>(self) -> Result<Ref<T>, RuntimeError>
    where
        T: Any,
    {
        let value = match self.into_repr()? {
            ValueRepr::Mutable(value) => value.into_ref()?,
            ValueRepr::Inline(actual) => {
                return Err(RuntimeError::expected_any(actual.type_info()));
            }
        };

        let result = Ref::try_map(value, |value| match value {
            Mutable::Any(any) => Some(any),
            _ => None,
        });

        let any = match result {
            Ok(any) => any,
            Err(actual) => return Err(RuntimeError::expected_any(actual.type_info())),
        };

        let result = Ref::result_map(any, |any| any.downcast_borrow_ref());

        match result {
            Ok(value) => Ok(value),
            Err((AnyObjError::Cast, any)) => {
                Err(RuntimeError::from(AccessErrorKind::UnexpectedType {
                    expected: any::type_name::<T>().into(),
                    actual: any.type_name(),
                }))
            }
            Err((error, _)) => Err(RuntimeError::from(AccessError::from(error))),
        }
    }

    /// Try to coerce value into a typed mutable reference.
    #[inline]
    pub fn into_any_mut<T>(self) -> Result<Mut<T>, RuntimeError>
    where
        T: Any,
    {
        let value = match self.into_repr()? {
            ValueRepr::Mutable(value) => value.into_mut()?,
            ValueRepr::Inline(actual) => {
                return Err(RuntimeError::expected_any(actual.type_info()));
            }
        };

        let result = Mut::try_map(value, |value| match value {
            Mutable::Any(any) => Some(any),
            _ => None,
        });

        let any = match result {
            Ok(any) => any,
            Err(actual) => return Err(RuntimeError::expected_any(actual.type_info())),
        };

        let result = Mut::result_map(any, |any| any.downcast_borrow_mut());

        match result {
            Ok(value) => Ok(value),
            Err((AnyObjError::Cast, any)) => {
                Err(RuntimeError::from(AccessErrorKind::UnexpectedType {
                    expected: any::type_name::<T>().into(),
                    actual: any.type_name(),
                }))
            }
            Err((error, _)) => Err(RuntimeError::from(AccessError::from(error))),
        }
    }

    /// Borrow the value as a typed reference.
    #[inline]
    pub fn borrow_any_ref<T>(&self) -> Result<BorrowRef<'_, T>, RuntimeError>
    where
        T: Any,
    {
        let value = match self.value_ref()? {
            ValueRef::Mutable(value) => value.borrow_ref()?,
            ValueRef::Inline(actual) => {
                return Err(RuntimeError::expected_any(actual.type_info()));
            }
        };

        let result = BorrowRef::try_map(value, |value| match value {
            Mutable::Any(any) => any.downcast_borrow_ref().ok(),
            _ => None,
        });

        match result {
            Ok(s) => Ok(s),
            Err(actual) => Err(RuntimeError::expected_any(actual.type_info())),
        }
    }

    /// Borrow the value as a mutable typed reference.
    #[inline]
    pub fn borrow_any_mut<T>(&self) -> Result<BorrowMut<'_, T>, RuntimeError>
    where
        T: Any,
    {
        let value = match self.value_ref()? {
            ValueRef::Mutable(value) => value.borrow_mut()?,
            ValueRef::Inline(actual) => {
                return Err(RuntimeError::expected_any(actual.type_info()));
            }
        };

        let result = BorrowMut::try_map(value, |value| match value {
            Mutable::Any(any) => any.downcast_borrow_mut().ok(),
            _ => None,
        });

        match result {
            Ok(s) => Ok(s),
            Err(actual) => Err(RuntimeError::expected_any(actual.type_info())),
        }
    }

    /// Try to coerce value into a typed value.
    #[inline]
    pub fn into_any<T>(self) -> Result<T, RuntimeError>
    where
        T: Any,
    {
        let any = match self.take_value()? {
            OwnedValue::Mutable(Mutable::Any(any)) => any,
            actual => return Err(RuntimeError::expected_any(actual.type_info())),
        };

        match any.downcast::<T>() {
            Ok(any) => Ok(any),
            Err((AnyObjError::Cast, any)) => {
                Err(RuntimeError::from(AccessErrorKind::UnexpectedType {
                    expected: any::type_name::<T>().into(),
                    actual: any.type_name(),
                }))
            }
            Err((error, _)) => Err(RuntimeError::from(AccessError::from(error))),
        }
    }

    /// Get the type hash for the current value.
    ///
    /// One notable feature is that the type of a variant is its container
    /// *enum*, and not the type hash of the variant itself.
    pub fn type_hash(&self) -> Result<Hash, AccessError> {
        match &self.repr {
            Repr::Inline(value) => Ok(value.type_hash()),
            Repr::Mutable(value) => Ok(value.borrow_ref()?.type_hash()),
            Repr::Empty => Err(AccessError::empty()),
        }
    }

    /// Get the type information for the current value.
    pub fn type_info(&self) -> Result<TypeInfo, AccessError> {
        match &self.repr {
            Repr::Inline(value) => Ok(value.type_info()),
            Repr::Mutable(value) => Ok(value.borrow_ref()?.type_info()),
            Repr::Empty => Err(AccessError::empty()),
        }
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
        Self::partial_eq_with(a, b, &mut EnvProtocolCaller)
    }

    /// Perform a total equality test between two values.
    ///
    /// This is the basis for the eq operation (`partial_eq` / '==').
    #[cfg_attr(feature = "bench", inline(never))]
    pub(crate) fn partial_eq_with(
        &self,
        b: &Value,
        caller: &mut dyn ProtocolCaller,
    ) -> VmResult<bool> {
        'fallback: {
            let a = vm_try!(self.borrow_ref());

            let a = match (&a, vm_try!(b.borrow_ref())) {
                (ValueBorrowRef::Inline(a), ValueBorrowRef::Inline(b)) => {
                    return a.partial_eq(b);
                }
                (ValueBorrowRef::Inline(lhs), rhs) => {
                    return err(VmErrorKind::UnsupportedBinaryOperation {
                        op: Protocol::PARTIAL_EQ.name,
                        lhs: lhs.type_info(),
                        rhs: rhs.type_info(),
                    });
                }
                (ValueBorrowRef::Mutable(a), ValueBorrowRef::Mutable(b2)) => match (&**a, &*b2) {
                    (Mutable::Bytes(a), Mutable::Bytes(b)) => {
                        return VmResult::Ok(*a == *b);
                    }
                    (Mutable::RangeFrom(a), Mutable::RangeFrom(b)) => {
                        return RangeFrom::partial_eq_with(a, b, caller);
                    }
                    (Mutable::RangeFull(a), Mutable::RangeFull(b)) => {
                        return RangeFull::partial_eq_with(a, b, caller);
                    }
                    (Mutable::RangeInclusive(a), Mutable::RangeInclusive(b)) => {
                        return RangeInclusive::partial_eq_with(a, b, caller);
                    }
                    (Mutable::RangeToInclusive(a), Mutable::RangeToInclusive(b)) => {
                        return RangeToInclusive::partial_eq_with(a, b, caller);
                    }
                    (Mutable::RangeTo(a), Mutable::RangeTo(b)) => {
                        return RangeTo::partial_eq_with(a, b, caller);
                    }
                    (Mutable::Range(a), Mutable::Range(b)) => {
                        return Range::partial_eq_with(a, b, caller);
                    }
                    (Mutable::ControlFlow(a), Mutable::ControlFlow(b)) => {
                        return ControlFlow::partial_eq_with(a, b, caller);
                    }
                    (Mutable::EmptyStruct(a), Mutable::EmptyStruct(b)) => {
                        if a.rtti.hash == b.rtti.hash {
                            // NB: don't get any future ideas, this must fall through to
                            // the VmError below since it's otherwise a comparison
                            // between two incompatible types.
                            //
                            // Other than that, all units are equal.
                            return VmResult::Ok(true);
                        }

                        break 'fallback;
                    }
                    (Mutable::TupleStruct(a), Mutable::TupleStruct(b)) => {
                        if a.rtti.hash == b.rtti.hash {
                            return Vec::eq_with(&a.data, &b.data, Value::partial_eq_with, caller);
                        }

                        break 'fallback;
                    }
                    (Mutable::Struct(a), Mutable::Struct(b)) => {
                        if a.rtti.hash == b.rtti.hash {
                            return Object::eq_with(
                                &a.data,
                                &b.data,
                                Value::partial_eq_with,
                                caller,
                            );
                        }

                        break 'fallback;
                    }
                    (Mutable::Variant(a), Mutable::Variant(b)) => {
                        if a.rtti().enum_hash == b.rtti().enum_hash {
                            return Variant::partial_eq_with(a, b, caller);
                        }

                        break 'fallback;
                    }
                    (Mutable::String(a), Mutable::String(b)) => {
                        return VmResult::Ok(*a == *b);
                    }
                    (Mutable::Option(a), Mutable::Option(b)) => match (a, b) {
                        (Some(a), Some(b)) => return Value::partial_eq_with(a, b, caller),
                        (None, None) => return VmResult::Ok(true),
                        _ => return VmResult::Ok(false),
                    },
                    (Mutable::Result(a), Mutable::Result(b)) => match (a, b) {
                        (Ok(a), Ok(b)) => return Value::partial_eq_with(a, b, caller),
                        (Err(a), Err(b)) => return Value::partial_eq_with(a, b, caller),
                        _ => return VmResult::Ok(false),
                    },
                    (a, _) => a,
                },
                _ => break 'fallback,
            };

            // Special cases.
            match a {
                Mutable::Vec(a) => {
                    return Vec::partial_eq_with(a, b.clone(), caller);
                }
                Mutable::Tuple(a) => {
                    return Vec::partial_eq_with(a, b.clone(), caller);
                }
                _ => {}
            }
        }

        if let CallResultOnly::Ok(value) = vm_try!(caller.try_call_protocol_fn(
            Protocol::PARTIAL_EQ,
            self.clone(),
            &mut Some((b.clone(),))
        )) {
            return <_>::from_value(value);
        }

        err(VmErrorKind::UnsupportedBinaryOperation {
            op: Protocol::PARTIAL_EQ.name,
            lhs: vm_try!(self.type_info()),
            rhs: vm_try!(b.type_info()),
        })
    }

    /// Hash the current value.
    #[cfg(feature = "alloc")]
    pub fn hash(&self, hasher: &mut Hasher) -> VmResult<()> {
        self.hash_with(hasher, &mut EnvProtocolCaller)
    }

    /// Hash the current value.
    #[cfg_attr(feature = "bench", inline(never))]
    pub(crate) fn hash_with(
        &self,
        hasher: &mut Hasher,
        caller: &mut dyn ProtocolCaller,
    ) -> VmResult<()> {
        match vm_try!(self.borrow_ref()) {
            ValueBorrowRef::Inline(value) => match value {
                Inline::Integer(value) => {
                    hasher.write_i64(*value);
                    return VmResult::Ok(());
                }
                Inline::Byte(value) => {
                    hasher.write_u8(*value);
                    return VmResult::Ok(());
                }
                // Care must be taken whan hashing floats, to ensure that `hash(v1)
                // === hash(v2)` if `eq(v1) === eq(v2)`. Hopefully we accomplish
                // this by rejecting NaNs and rectifying subnormal values of zero.
                Inline::Float(value) => {
                    if value.is_nan() {
                        return VmResult::err(VmErrorKind::IllegalFloatOperation { value: *value });
                    }

                    let zero = *value == 0.0;
                    hasher.write_f64((zero as u8 as f64) * 0.0 + (!zero as u8 as f64) * *value);
                    return VmResult::Ok(());
                }
                operand => {
                    return err(VmErrorKind::UnsupportedUnaryOperation {
                        op: Protocol::HASH.name,
                        operand: operand.type_info(),
                    });
                }
            },
            ValueBorrowRef::Mutable(value) => match &*value {
                Mutable::String(string) => {
                    hasher.write_str(string);
                    return VmResult::Ok(());
                }
                Mutable::Bytes(bytes) => {
                    hasher.write(bytes);
                    return VmResult::Ok(());
                }
                Mutable::Tuple(tuple) => {
                    return Tuple::hash_with(tuple, hasher, caller);
                }
                Mutable::Vec(vec) => {
                    return Vec::hash_with(vec, hasher, caller);
                }
                _ => {}
            },
        }

        let mut args = DynGuardedArgs::new((hasher,));

        if let CallResultOnly::Ok(value) =
            vm_try!(caller.try_call_protocol_fn(Protocol::HASH, self.clone(), &mut args))
        {
            return <_>::from_value(value);
        }

        err(VmErrorKind::UnsupportedUnaryOperation {
            op: Protocol::HASH.name,
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
    #[cfg_attr(feature = "bench", inline(never))]
    pub(crate) fn eq_with(&self, b: &Value, caller: &mut dyn ProtocolCaller) -> VmResult<bool> {
        match (vm_try!(self.borrow_ref()), vm_try!(b.borrow_ref())) {
            (ValueBorrowRef::Inline(a), ValueBorrowRef::Inline(b)) => {
                return a.eq(b);
            }
            (ValueBorrowRef::Inline(lhs), rhs) => {
                return err(VmErrorKind::UnsupportedBinaryOperation {
                    op: Protocol::EQ.name,
                    lhs: lhs.type_info(),
                    rhs: rhs.type_info(),
                });
            }
            (ValueBorrowRef::Mutable(a), ValueBorrowRef::Mutable(b)) => match (&*a, &*b) {
                (Mutable::Bytes(a), Mutable::Bytes(b)) => {
                    return VmResult::Ok(*a == *b);
                }
                (Mutable::Vec(a), Mutable::Vec(b)) => {
                    return Vec::eq_with(a, b, Value::eq_with, caller);
                }
                (Mutable::Tuple(a), Mutable::Tuple(b)) => {
                    return Vec::eq_with(a, b, Value::eq_with, caller);
                }
                (Mutable::Object(a), Mutable::Object(b)) => {
                    return Object::eq_with(a, b, Value::eq_with, caller);
                }
                (Mutable::RangeFrom(a), Mutable::RangeFrom(b)) => {
                    return RangeFrom::eq_with(a, b, caller);
                }
                (Mutable::RangeFull(a), Mutable::RangeFull(b)) => {
                    return RangeFull::eq_with(a, b, caller);
                }
                (Mutable::RangeInclusive(a), Mutable::RangeInclusive(b)) => {
                    return RangeInclusive::eq_with(a, b, caller);
                }
                (Mutable::RangeToInclusive(a), Mutable::RangeToInclusive(b)) => {
                    return RangeToInclusive::eq_with(a, b, caller);
                }
                (Mutable::RangeTo(a), Mutable::RangeTo(b)) => {
                    return RangeTo::eq_with(a, b, caller);
                }
                (Mutable::Range(a), Mutable::Range(b)) => {
                    return Range::eq_with(a, b, caller);
                }
                (Mutable::ControlFlow(a), Mutable::ControlFlow(b)) => {
                    return ControlFlow::eq_with(a, b, caller);
                }
                (Mutable::EmptyStruct(a), Mutable::EmptyStruct(b)) => {
                    if a.rtti.hash == b.rtti.hash {
                        // NB: don't get any future ideas, this must fall through to
                        // the VmError below since it's otherwise a comparison
                        // between two incompatible types.
                        //
                        // Other than that, all units are equal.
                        return VmResult::Ok(true);
                    }
                }
                (Mutable::TupleStruct(a), Mutable::TupleStruct(b)) => {
                    if a.rtti.hash == b.rtti.hash {
                        return Vec::eq_with(&a.data, &b.data, Value::eq_with, caller);
                    }
                }
                (Mutable::Struct(a), Mutable::Struct(b)) => {
                    if a.rtti.hash == b.rtti.hash {
                        return Object::eq_with(&a.data, &b.data, Value::eq_with, caller);
                    }
                }
                (Mutable::Variant(a), Mutable::Variant(b)) => {
                    if a.rtti().enum_hash == b.rtti().enum_hash {
                        return Variant::eq_with(a, b, caller);
                    }
                }
                (Mutable::String(a), Mutable::String(b)) => {
                    return VmResult::Ok(*a == *b);
                }
                (Mutable::Option(a), Mutable::Option(b)) => match (a, b) {
                    (Some(a), Some(b)) => return Value::eq_with(a, b, caller),
                    (None, None) => return VmResult::Ok(true),
                    _ => return VmResult::Ok(false),
                },
                (Mutable::Result(a), Mutable::Result(b)) => match (a, b) {
                    (Ok(a), Ok(b)) => return Value::eq_with(a, b, caller),
                    (Err(a), Err(b)) => return Value::eq_with(a, b, caller),
                    _ => return VmResult::Ok(false),
                },
                _ => {}
            },
            _ => {}
        }

        if let CallResultOnly::Ok(value) = vm_try!(caller.try_call_protocol_fn(
            Protocol::EQ,
            self.clone(),
            &mut Some((b.clone(),))
        )) {
            return <_>::from_value(value);
        }

        err(VmErrorKind::UnsupportedBinaryOperation {
            op: Protocol::EQ.name,
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
    #[cfg_attr(feature = "bench", inline(never))]
    pub(crate) fn partial_cmp_with(
        &self,
        b: &Value,
        caller: &mut dyn ProtocolCaller,
    ) -> VmResult<Option<Ordering>> {
        match (vm_try!(self.borrow_ref()), vm_try!(b.borrow_ref())) {
            (ValueBorrowRef::Inline(a), ValueBorrowRef::Inline(b)) => return a.partial_cmp(b),
            (ValueBorrowRef::Inline(lhs), rhs) => {
                return err(VmErrorKind::UnsupportedBinaryOperation {
                    op: Protocol::PARTIAL_CMP.name,
                    lhs: lhs.type_info(),
                    rhs: rhs.type_info(),
                })
            }
            (ValueBorrowRef::Mutable(a), ValueBorrowRef::Mutable(b)) => match (&*a, &*b) {
                (Mutable::Bytes(a), Mutable::Bytes(b)) => {
                    return VmResult::Ok(a.partial_cmp(b));
                }
                (Mutable::Vec(a), Mutable::Vec(b)) => {
                    return Vec::partial_cmp_with(a, b, caller);
                }
                (Mutable::Tuple(a), Mutable::Tuple(b)) => {
                    return Vec::partial_cmp_with(a, b, caller);
                }
                (Mutable::Object(a), Mutable::Object(b)) => {
                    return Object::partial_cmp_with(a, b, caller);
                }
                (Mutable::RangeFrom(a), Mutable::RangeFrom(b)) => {
                    return RangeFrom::partial_cmp_with(a, b, caller);
                }
                (Mutable::RangeFull(a), Mutable::RangeFull(b)) => {
                    return RangeFull::partial_cmp_with(a, b, caller);
                }
                (Mutable::RangeInclusive(a), Mutable::RangeInclusive(b)) => {
                    return RangeInclusive::partial_cmp_with(a, b, caller);
                }
                (Mutable::RangeToInclusive(a), Mutable::RangeToInclusive(b)) => {
                    return RangeToInclusive::partial_cmp_with(a, b, caller);
                }
                (Mutable::RangeTo(a), Mutable::RangeTo(b)) => {
                    return RangeTo::partial_cmp_with(a, b, caller);
                }
                (Mutable::Range(a), Mutable::Range(b)) => {
                    return Range::partial_cmp_with(a, b, caller);
                }
                (Mutable::EmptyStruct(a), Mutable::EmptyStruct(b)) => {
                    if a.rtti.hash == b.rtti.hash {
                        // NB: don't get any future ideas, this must fall through to
                        // the VmError below since it's otherwise a comparison
                        // between two incompatible types.
                        //
                        // Other than that, all units are equal.
                        return VmResult::Ok(Some(Ordering::Equal));
                    }
                }
                (Mutable::TupleStruct(a), Mutable::TupleStruct(b)) => {
                    if a.rtti.hash == b.rtti.hash {
                        return Vec::partial_cmp_with(&a.data, &b.data, caller);
                    }
                }
                (Mutable::Struct(a), Mutable::Struct(b)) => {
                    if a.rtti.hash == b.rtti.hash {
                        return Object::partial_cmp_with(&a.data, &b.data, caller);
                    }
                }
                (Mutable::Variant(a), Mutable::Variant(b)) => {
                    if a.rtti().enum_hash == b.rtti().enum_hash {
                        return Variant::partial_cmp_with(a, b, caller);
                    }
                }
                (Mutable::String(a), Mutable::String(b)) => {
                    return VmResult::Ok(a.partial_cmp(b));
                }
                (Mutable::Option(a), Mutable::Option(b)) => match (a, b) {
                    (Some(a), Some(b)) => return Value::partial_cmp_with(a, b, caller),
                    (None, None) => return VmResult::Ok(Some(Ordering::Equal)),
                    (Some(..), None) => return VmResult::Ok(Some(Ordering::Greater)),
                    (None, Some(..)) => return VmResult::Ok(Some(Ordering::Less)),
                },
                (Mutable::Result(a), Mutable::Result(b)) => match (a, b) {
                    (Ok(a), Ok(b)) => return Value::partial_cmp_with(a, b, caller),
                    (Err(a), Err(b)) => return Value::partial_cmp_with(a, b, caller),
                    (Ok(..), Err(..)) => return VmResult::Ok(Some(Ordering::Greater)),
                    (Err(..), Ok(..)) => return VmResult::Ok(Some(Ordering::Less)),
                },
                _ => {}
            },
            _ => {}
        }

        if let CallResultOnly::Ok(value) = vm_try!(caller.try_call_protocol_fn(
            Protocol::PARTIAL_CMP,
            self.clone(),
            &mut Some((b.clone(),))
        )) {
            return <_>::from_value(value);
        }

        err(VmErrorKind::UnsupportedBinaryOperation {
            op: Protocol::PARTIAL_CMP.name,
            lhs: vm_try!(self.type_info()),
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
    #[cfg_attr(feature = "bench", inline(never))]
    pub(crate) fn cmp_with(
        &self,
        b: &Value,
        caller: &mut dyn ProtocolCaller,
    ) -> VmResult<Ordering> {
        match (vm_try!(self.borrow_ref()), vm_try!(b.borrow_ref())) {
            (ValueBorrowRef::Inline(a), ValueBorrowRef::Inline(b)) => return a.cmp(b),
            (ValueBorrowRef::Mutable(a), ValueBorrowRef::Mutable(b)) => match (&*a, &*b) {
                (Mutable::Bytes(a), Mutable::Bytes(b)) => {
                    return VmResult::Ok(a.cmp(b));
                }
                (Mutable::Vec(a), Mutable::Vec(b)) => {
                    return Vec::cmp_with(a, b, caller);
                }
                (Mutable::Tuple(a), Mutable::Tuple(b)) => {
                    return Vec::cmp_with(a, b, caller);
                }
                (Mutable::Object(a), Mutable::Object(b)) => {
                    return Object::cmp_with(a, b, caller);
                }
                (Mutable::RangeFrom(a), Mutable::RangeFrom(b)) => {
                    return RangeFrom::cmp_with(a, b, caller);
                }
                (Mutable::RangeFull(a), Mutable::RangeFull(b)) => {
                    return RangeFull::cmp_with(a, b, caller);
                }
                (Mutable::RangeInclusive(a), Mutable::RangeInclusive(b)) => {
                    return RangeInclusive::cmp_with(a, b, caller);
                }
                (Mutable::RangeToInclusive(a), Mutable::RangeToInclusive(b)) => {
                    return RangeToInclusive::cmp_with(a, b, caller);
                }
                (Mutable::RangeTo(a), Mutable::RangeTo(b)) => {
                    return RangeTo::cmp_with(a, b, caller);
                }
                (Mutable::Range(a), Mutable::Range(b)) => {
                    return Range::cmp_with(a, b, caller);
                }
                (Mutable::EmptyStruct(a), Mutable::EmptyStruct(b)) => {
                    if a.rtti.hash == b.rtti.hash {
                        // NB: don't get any future ideas, this must fall through to
                        // the VmError below since it's otherwise a comparison
                        // between two incompatible types.
                        //
                        // Other than that, all units are equal.
                        return VmResult::Ok(Ordering::Equal);
                    }
                }
                (Mutable::TupleStruct(a), Mutable::TupleStruct(b)) => {
                    if a.rtti.hash == b.rtti.hash {
                        return Vec::cmp_with(&a.data, &b.data, caller);
                    }
                }
                (Mutable::Struct(a), Mutable::Struct(b)) => {
                    if a.rtti.hash == b.rtti.hash {
                        return Object::cmp_with(&a.data, &b.data, caller);
                    }
                }
                (Mutable::Variant(a), Mutable::Variant(b)) => {
                    if a.rtti().enum_hash == b.rtti().enum_hash {
                        return Variant::cmp_with(a, b, caller);
                    }
                }
                (Mutable::String(a), Mutable::String(b)) => {
                    return VmResult::Ok(a.cmp(b));
                }
                (Mutable::Option(a), Mutable::Option(b)) => match (a, b) {
                    (Some(a), Some(b)) => return Value::cmp_with(a, b, caller),
                    (None, None) => return VmResult::Ok(Ordering::Equal),
                    (Some(..), None) => return VmResult::Ok(Ordering::Greater),
                    (None, Some(..)) => return VmResult::Ok(Ordering::Less),
                },
                (Mutable::Result(a), Mutable::Result(b)) => match (a, b) {
                    (Ok(a), Ok(b)) => return Value::cmp_with(a, b, caller),
                    (Err(a), Err(b)) => return Value::cmp_with(a, b, caller),
                    (Ok(..), Err(..)) => return VmResult::Ok(Ordering::Greater),
                    (Err(..), Ok(..)) => return VmResult::Ok(Ordering::Less),
                },
                _ => {}
            },
            (ValueBorrowRef::Inline(lhs), rhs) => {
                return VmResult::err(VmErrorKind::UnsupportedBinaryOperation {
                    op: Protocol::CMP.name,
                    lhs: lhs.type_info(),
                    rhs: rhs.type_info(),
                });
            }
            _ => {}
        }

        if let CallResultOnly::Ok(value) = vm_try!(caller.try_call_protocol_fn(
            Protocol::CMP,
            self.clone(),
            &mut Some((b.clone(),))
        )) {
            return <_>::from_value(value);
        }

        err(VmErrorKind::UnsupportedBinaryOperation {
            op: Protocol::CMP.name,
            lhs: vm_try!(self.type_info()),
            rhs: vm_try!(b.type_info()),
        })
    }

    /// Try to coerce the current value as the specified integer `T`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::runtime::{Value, VmResult};
    ///
    /// let value = rune::to_value(u32::MAX)?;
    ///
    /// assert_eq!(value.try_as_integer::<u64>(), Ok(u32::MAX as u64));
    /// assert!(value.try_as_integer::<i32>().is_err());
    ///
    /// # Ok::<(), rune::support::Error>(())
    /// ```
    pub fn try_as_integer<T>(&self) -> Result<T, RuntimeError>
    where
        T: TryFrom<i64>,
        VmIntegerRepr: From<i64>,
    {
        let integer = self.as_integer()?;

        match integer.try_into() {
            Ok(number) => Ok(number),
            Err(..) => Err(RuntimeError::new(
                VmErrorKind::ValueToIntegerCoercionError {
                    from: VmIntegerRepr::from(integer),
                    to: any::type_name::<T>(),
                },
            )),
        }
    }

    pub(crate) fn as_inline_unchecked(&self) -> Option<&Inline> {
        match &self.repr {
            Repr::Inline(value) => Some(value),
            _ => None,
        }
    }

    pub(crate) fn as_inline(&self) -> Result<Option<&Inline>, AccessError> {
        match &self.repr {
            Repr::Inline(value) => Ok(Some(value)),
            Repr::Mutable(..) => Ok(None),
            Repr::Empty => Err(AccessError::empty()),
        }
    }

    pub(crate) fn as_inline_mut(&mut self) -> Result<Option<&mut Inline>, AccessError> {
        match &mut self.repr {
            Repr::Inline(value) => Ok(Some(value)),
            Repr::Mutable(..) => Ok(None),
            Repr::Empty => Err(AccessError::empty()),
        }
    }

    pub(crate) fn take_value(self) -> Result<OwnedValue, AccessError> {
        match self.repr {
            Repr::Inline(value) => Ok(OwnedValue::Inline(value)),
            Repr::Mutable(value) => Ok(OwnedValue::Mutable(value.take()?)),
            Repr::Empty => Err(AccessError::empty()),
        }
    }

    pub(crate) fn into_repr(self) -> Result<ValueRepr, AccessError> {
        match self.repr {
            Repr::Inline(value) => Ok(ValueRepr::Inline(value)),
            Repr::Mutable(value) => Ok(ValueRepr::Mutable(value)),
            Repr::Empty => Err(AccessError::empty()),
        }
    }

    pub(crate) fn borrow_ref(&self) -> Result<ValueBorrowRef<'_>, AccessError> {
        match &self.repr {
            Repr::Inline(value) => Ok(ValueBorrowRef::Inline(value)),
            Repr::Mutable(value) => Ok(ValueBorrowRef::Mutable(value.borrow_ref()?)),
            Repr::Empty => Err(AccessError::empty()),
        }
    }

    pub(crate) fn value_ref(&self) -> Result<ValueRef<'_>, AccessError> {
        match &self.repr {
            Repr::Inline(value) => Ok(ValueRef::Inline(value)),
            Repr::Mutable(value) => Ok(ValueRef::Mutable(value)),
            Repr::Empty => Err(AccessError::empty()),
        }
    }

    pub(crate) fn value_mut(&mut self) -> Result<ValueMut<'_>, AccessError> {
        match &mut self.repr {
            Repr::Inline(value) => Ok(ValueMut::Inline(value)),
            Repr::Mutable(mutable) => Ok(ValueMut::Mutable(mutable)),
            Repr::Empty => Err(AccessError::empty()),
        }
    }

    pub(crate) fn into_value_shared(self) -> Result<ValueShared, AccessError> {
        match self.repr {
            Repr::Inline(value) => Ok(ValueShared::Inline(value)),
            Repr::Mutable(value) => Ok(ValueShared::Mutable(value)),
            Repr::Empty => Err(AccessError::empty()),
        }
    }

    pub(crate) fn protocol_into_iter(&self) -> VmResult<Value> {
        EnvProtocolCaller.call_protocol_fn(Protocol::INTO_ITER, self.clone(), &mut ())
    }

    pub(crate) fn protocol_next(&self) -> VmResult<Option<Value>> {
        let value =
            vm_try!(EnvProtocolCaller.call_protocol_fn(Protocol::NEXT, self.clone(), &mut ()));

        FromValue::from_value(value)
    }

    pub(crate) fn protocol_next_back(&self) -> VmResult<Option<Value>> {
        let value =
            vm_try!(EnvProtocolCaller.call_protocol_fn(Protocol::NEXT_BACK, self.clone(), &mut ()));

        FromValue::from_value(value)
    }

    pub(crate) fn protocol_nth_back(&self, n: usize) -> VmResult<Option<Value>> {
        let value = vm_try!(EnvProtocolCaller.call_protocol_fn(
            Protocol::NTH_BACK,
            self.clone(),
            &mut Some((n,))
        ));

        FromValue::from_value(value)
    }

    pub(crate) fn protocol_len(&self) -> VmResult<usize> {
        let value =
            vm_try!(EnvProtocolCaller.call_protocol_fn(Protocol::LEN, self.clone(), &mut ()));

        FromValue::from_value(value)
    }

    pub(crate) fn protocol_size_hint(&self) -> VmResult<(usize, Option<usize>)> {
        let value =
            vm_try!(EnvProtocolCaller.call_protocol_fn(Protocol::SIZE_HINT, self.clone(), &mut ()));

        FromValue::from_value(value)
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let snapshot = match &self.repr {
            Repr::Empty => {
                write!(f, "<empty>")?;
                return Ok(());
            }
            Repr::Inline(value) => {
                write!(f, "{value:?}")?;
                return Ok(());
            }
            Repr::Mutable(value) => value.snapshot(),
        };

        if !snapshot.is_readable() {
            write!(f, "<{snapshot}>")?;
            return Ok(());
        }

        let mut o = Formatter::new();

        if let Err(e) = self.string_debug(&mut o).into_result() {
            match &self.repr {
                Repr::Empty => {
                    write!(f, "<empty: {e}>")?;
                }
                Repr::Inline(value) => {
                    write!(f, "<{value:?}: {e}>")?;
                }
                Repr::Mutable(value) => match value.borrow_ref() {
                    Ok(v) => {
                        let ty = v.type_info();
                        write!(f, "<{ty} object at {value:p}: {e}>")?;
                    }
                    Err(e2) => {
                        write!(f, "<unknown object at {value:p}: {e}: {e2}>")?;
                    }
                },
            }

            return Ok(());
        }

        f.write_str(o.as_str())?;
        Ok(())
    }
}

impl From<()> for Value {
    #[inline]
    fn from((): ()) -> Self {
        Value::from(Inline::Unit)
    }
}

impl IntoOutput for () {
    type Output = ();

    #[inline]
    fn into_output(self) -> VmResult<Self::Output> {
        VmResult::Ok(())
    }
}

impl From<Inline> for Value {
    #[inline]
    fn from(value: Inline) -> Self {
        Self {
            repr: Repr::Inline(value),
        }
    }
}

impl IntoOutput for Inline {
    type Output = Inline;

    #[inline]
    fn into_output(self) -> VmResult<Self::Output> {
        VmResult::Ok(self)
    }
}

impl TryFrom<Mutable> for Value {
    type Error = alloc::Error;

    #[inline]
    fn try_from(value: Mutable) -> Result<Self, Self::Error> {
        Ok(Self {
            repr: Repr::Mutable(Shared::new(value)?),
        })
    }
}

impl IntoOutput for Mutable {
    type Output = Mutable;

    #[inline]
    fn into_output(self) -> VmResult<Self::Output> {
        VmResult::Ok(self)
    }
}

impl ToValue for Value {
    #[inline]
    fn to_value(self) -> VmResult<Value> {
        VmResult::Ok(self)
    }
}

inline_from! {
    Byte => u8,
    Bool => bool,
    Char => char,
    Integer => i64,
    Float => f64,
    Type => Type,
    Ordering => Ordering,
}

from! {
    String => String,
    Bytes => Bytes,
    ControlFlow => ControlFlow,
    Function => Function,
    GeneratorState => GeneratorState,
    Vec => Vec,
    EmptyStruct => EmptyStruct,
    TupleStruct => TupleStruct,
    Struct => Struct,
    Variant => Variant,
    Object => Object,
    Tuple => OwnedTuple,
    Generator => Generator<Vm>,
    Format => Format,
    RangeFrom => RangeFrom,
    RangeFull => RangeFull,
    RangeInclusive => RangeInclusive,
    RangeToInclusive => RangeToInclusive,
    RangeTo => RangeTo,
    Range => Range,
    Future => Future,
    Stream => Stream<Vm>,
    Any => AnyObj,
}

from_container! {
    Option => Option<Value>,
    Result => Result<Value, Value>,
}

impl MaybeTypeOf for Value {
    #[inline]
    fn maybe_type_of() -> alloc::Result<meta::DocType> {
        Ok(meta::DocType::empty())
    }
}

impl Clone for Value {
    #[inline]
    fn clone(&self) -> Self {
        let repr = match &self.repr {
            Repr::Empty => Repr::Empty,
            Repr::Inline(inline) => Repr::Inline(*inline),
            Repr::Mutable(mutable) => Repr::Mutable(mutable.clone()),
        };

        Self { repr }
    }

    #[inline]
    fn clone_from(&mut self, source: &Self) {
        match (&mut self.repr, &source.repr) {
            (Repr::Empty, Repr::Empty) => {}
            (Repr::Inline(lhs), Repr::Inline(rhs)) => {
                *lhs = *rhs;
            }
            (Repr::Mutable(lhs), Repr::Mutable(rhs)) => {
                lhs.clone_from(rhs);
            }
            (lhs, rhs) => {
                *lhs = rhs.clone();
            }
        }
    }
}

impl TryClone for Value {
    fn try_clone(&self) -> alloc::Result<Self> {
        // NB: value cloning is a shallow clone of the underlying data.
        Ok(self.clone())
    }
}

/// Wrapper for a value kind.
#[doc(hidden)]
pub struct NotTypedInlineValue(Inline);

/// Wrapper for a value kind.
#[doc(hidden)]
pub struct NotTypedMutableValue(Mutable);

/// The coersion of a value into a typed value.
#[doc(hidden)]
#[non_exhaustive]
pub enum TypeValue {
    /// The unit value.
    Unit,
    /// A tuple.
    Tuple(OwnedTuple),
    /// An object.
    Object(Object),
    /// An struct with a well-defined type.
    EmptyStruct(EmptyStruct),
    /// A tuple with a well-defined type.
    TupleStruct(TupleStruct),
    /// An struct with a well-defined type.
    Struct(Struct),
    /// The variant of an enum.
    Variant(Variant),
    /// Not a typed immutable value.
    #[doc(hidden)]
    NotTypedInline(NotTypedInlineValue),
    /// Not a typed value.
    #[doc(hidden)]
    NotTypedMutable(NotTypedMutableValue),
}

impl TypeValue {
    /// Get the type info of the current value.
    #[doc(hidden)]
    pub fn type_info(&self) -> TypeInfo {
        match self {
            TypeValue::Unit => TypeInfo::StaticType(static_type::TUPLE),
            TypeValue::Tuple(..) => TypeInfo::StaticType(static_type::TUPLE),
            TypeValue::Object(..) => TypeInfo::StaticType(static_type::OBJECT),
            TypeValue::EmptyStruct(empty) => empty.type_info(),
            TypeValue::TupleStruct(tuple) => tuple.type_info(),
            TypeValue::Struct(object) => object.type_info(),
            TypeValue::Variant(empty) => empty.type_info(),
            TypeValue::NotTypedInline(value) => value.0.type_info(),
            TypeValue::NotTypedMutable(value) => value.0.type_info(),
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum Inline {
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
    Type(Type),
    /// Ordering.
    Ordering(Ordering),
}

impl Inline {
    /// Perform a partial equality check over two inline values.
    pub(crate) fn partial_eq(&self, other: &Self) -> VmResult<bool> {
        match (self, other) {
            (Inline::Unit, Inline::Unit) => VmResult::Ok(true),
            (Inline::Bool(a), Inline::Bool(b)) => VmResult::Ok(*a == *b),
            (Inline::Byte(a), Inline::Byte(b)) => VmResult::Ok(*a == *b),
            (Inline::Char(a), Inline::Char(b)) => VmResult::Ok(*a == *b),
            (Inline::Integer(a), Inline::Integer(b)) => VmResult::Ok(*a == *b),
            (Inline::Float(a), Inline::Float(b)) => VmResult::Ok(*a == *b),
            (Inline::Type(a), Inline::Type(b)) => VmResult::Ok(*a == *b),
            (Inline::Ordering(a), Inline::Ordering(b)) => VmResult::Ok(*a == *b),
            (lhs, rhs) => err(VmErrorKind::UnsupportedBinaryOperation {
                op: Protocol::PARTIAL_EQ.name,
                lhs: lhs.type_info(),
                rhs: rhs.type_info(),
            }),
        }
    }

    /// Perform a total equality check over two inline values.
    pub(crate) fn eq(&self, other: &Self) -> VmResult<bool> {
        match (self, other) {
            (Inline::Unit, Inline::Unit) => VmResult::Ok(true),
            (Inline::Bool(a), Inline::Bool(b)) => VmResult::Ok(*a == *b),
            (Inline::Byte(a), Inline::Byte(b)) => VmResult::Ok(*a == *b),
            (Inline::Char(a), Inline::Char(b)) => VmResult::Ok(*a == *b),
            (Inline::Float(a), Inline::Float(b)) => {
                let Some(ordering) = a.partial_cmp(b) else {
                    return VmResult::err(VmErrorKind::IllegalFloatComparison { lhs: *a, rhs: *b });
                };

                VmResult::Ok(matches!(ordering, Ordering::Equal))
            }
            (Inline::Integer(a), Inline::Integer(b)) => VmResult::Ok(*a == *b),
            (Inline::Type(a), Inline::Type(b)) => VmResult::Ok(*a == *b),
            (Inline::Ordering(a), Inline::Ordering(b)) => VmResult::Ok(*a == *b),
            (lhs, rhs) => err(VmErrorKind::UnsupportedBinaryOperation {
                op: Protocol::EQ.name,
                lhs: lhs.type_info(),
                rhs: rhs.type_info(),
            }),
        }
    }

    /// Partial comparison implementation for inline.
    pub(crate) fn partial_cmp(&self, other: &Self) -> VmResult<Option<Ordering>> {
        match (self, other) {
            (Inline::Unit, Inline::Unit) => VmResult::Ok(Some(Ordering::Equal)),
            (Inline::Bool(lhs), Inline::Bool(rhs)) => VmResult::Ok(lhs.partial_cmp(rhs)),
            (Inline::Byte(lhs), Inline::Byte(rhs)) => VmResult::Ok(lhs.partial_cmp(rhs)),
            (Inline::Char(lhs), Inline::Char(rhs)) => VmResult::Ok(lhs.partial_cmp(rhs)),
            (Inline::Float(lhs), Inline::Float(rhs)) => VmResult::Ok(lhs.partial_cmp(rhs)),
            (Inline::Integer(lhs), Inline::Integer(rhs)) => VmResult::Ok(lhs.partial_cmp(rhs)),
            (Inline::Type(lhs), Inline::Type(rhs)) => VmResult::Ok(lhs.partial_cmp(rhs)),
            (Inline::Ordering(lhs), Inline::Ordering(rhs)) => VmResult::Ok(lhs.partial_cmp(rhs)),
            (lhs, rhs) => err(VmErrorKind::UnsupportedBinaryOperation {
                op: Protocol::PARTIAL_CMP.name,
                lhs: lhs.type_info(),
                rhs: rhs.type_info(),
            }),
        }
    }

    /// Total comparison implementation for inline.
    pub(crate) fn cmp(&self, other: &Self) -> VmResult<Ordering> {
        match (self, other) {
            (Inline::Unit, Inline::Unit) => VmResult::Ok(Ordering::Equal),
            (Inline::Bool(a), Inline::Bool(b)) => VmResult::Ok(a.cmp(b)),
            (Inline::Byte(a), Inline::Byte(b)) => VmResult::Ok(a.cmp(b)),
            (Inline::Char(a), Inline::Char(b)) => VmResult::Ok(a.cmp(b)),
            (Inline::Float(a), Inline::Float(b)) => {
                let Some(ordering) = a.partial_cmp(b) else {
                    return VmResult::err(VmErrorKind::IllegalFloatComparison { lhs: *a, rhs: *b });
                };

                VmResult::Ok(ordering)
            }
            (Inline::Integer(a), Inline::Integer(b)) => VmResult::Ok(a.cmp(b)),
            (Inline::Type(a), Inline::Type(b)) => VmResult::Ok(a.cmp(b)),
            (Inline::Ordering(a), Inline::Ordering(b)) => VmResult::Ok(a.cmp(b)),
            (lhs, rhs) => VmResult::err(VmErrorKind::UnsupportedBinaryOperation {
                op: Protocol::CMP.name,
                lhs: lhs.type_info(),
                rhs: rhs.type_info(),
            }),
        }
    }
}

impl fmt::Debug for Inline {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Inline::Unit => write!(f, "()"),
            Inline::Bool(value) => value.fmt(f),
            Inline::Byte(value) => value.fmt(f),
            Inline::Char(value) => value.fmt(f),
            Inline::Integer(value) => value.fmt(f),
            Inline::Float(value) => value.fmt(f),
            Inline::Type(value) => value.fmt(f),
            Inline::Ordering(value) => value.fmt(f),
        }
    }
}

impl Inline {
    pub(crate) fn type_info(&self) -> TypeInfo {
        match self {
            Inline::Unit => TypeInfo::StaticType(static_type::TUPLE),
            Inline::Bool(..) => TypeInfo::StaticType(static_type::BOOL),
            Inline::Byte(..) => TypeInfo::StaticType(static_type::BYTE),
            Inline::Char(..) => TypeInfo::StaticType(static_type::CHAR),
            Inline::Integer(..) => TypeInfo::StaticType(static_type::INTEGER),
            Inline::Float(..) => TypeInfo::StaticType(static_type::FLOAT),
            Inline::Type(..) => TypeInfo::StaticType(static_type::TYPE),
            Inline::Ordering(..) => TypeInfo::StaticType(static_type::ORDERING),
        }
    }

    /// Get the type hash for the current value.
    ///
    /// One notable feature is that the type of a variant is its container
    /// *enum*, and not the type hash of the variant itself.
    pub(crate) fn type_hash(&self) -> Hash {
        match self {
            Inline::Unit => static_type::TUPLE.hash,
            Inline::Bool(..) => static_type::BOOL.hash,
            Inline::Byte(..) => static_type::BYTE.hash,
            Inline::Char(..) => static_type::CHAR.hash,
            Inline::Integer(..) => static_type::INTEGER.hash,
            Inline::Float(..) => static_type::FLOAT.hash,
            Inline::Type(..) => static_type::TYPE.hash,
            Inline::Ordering(..) => static_type::ORDERING.hash,
        }
    }
}

pub(crate) enum Mutable {
    /// A UTF-8 string.
    String(String),
    /// A byte string.
    Bytes(Bytes),
    /// A vector containing any values.
    Vec(Vec),
    /// A tuple.
    Tuple(OwnedTuple),
    /// An object.
    Object(Object),
    /// A range `start..`
    RangeFrom(RangeFrom),
    /// A full range `..`
    RangeFull(RangeFull),
    /// A full range `start..=end`
    RangeInclusive(RangeInclusive),
    /// A full range `..=end`
    RangeToInclusive(RangeToInclusive),
    /// A full range `..end`
    RangeTo(RangeTo),
    /// A range `start..end`.
    Range(Range),
    /// A control flow indicator.
    ControlFlow(ControlFlow),
    /// A stored future.
    Future(Future),
    /// A Stream.
    Stream(Stream<Vm>),
    /// A stored generator.
    Generator(Generator<Vm>),
    /// Generator state.
    GeneratorState(GeneratorState),
    /// An empty value indicating nothing.
    Option(Option<Value>),
    /// A stored result in a slot.
    Result(Result<Value, Value>),
    /// An struct with a well-defined type.
    EmptyStruct(EmptyStruct),
    /// A tuple with a well-defined type.
    TupleStruct(TupleStruct),
    /// An struct with a well-defined type.
    Struct(Struct),
    /// The variant of an enum.
    Variant(Variant),
    /// A stored function pointer.
    Function(Function),
    /// A value being formatted.
    Format(Format),
    /// An opaque value that can be downcasted.
    Any(AnyObj),
}

impl Mutable {
    pub(crate) fn type_info(&self) -> TypeInfo {
        match self {
            Mutable::String(..) => TypeInfo::StaticType(static_type::STRING),
            Mutable::Bytes(..) => TypeInfo::StaticType(static_type::BYTES),
            Mutable::Vec(..) => TypeInfo::StaticType(static_type::VEC),
            Mutable::Tuple(..) => TypeInfo::StaticType(static_type::TUPLE),
            Mutable::Object(..) => TypeInfo::StaticType(static_type::OBJECT),
            Mutable::RangeFrom(..) => TypeInfo::StaticType(static_type::RANGE_FROM),
            Mutable::RangeFull(..) => TypeInfo::StaticType(static_type::RANGE_FULL),
            Mutable::RangeInclusive(..) => TypeInfo::StaticType(static_type::RANGE_INCLUSIVE),
            Mutable::RangeToInclusive(..) => TypeInfo::StaticType(static_type::RANGE_TO_INCLUSIVE),
            Mutable::RangeTo(..) => TypeInfo::StaticType(static_type::RANGE_TO),
            Mutable::Range(..) => TypeInfo::StaticType(static_type::RANGE),
            Mutable::ControlFlow(..) => TypeInfo::StaticType(static_type::CONTROL_FLOW),
            Mutable::Future(..) => TypeInfo::StaticType(static_type::FUTURE),
            Mutable::Stream(..) => TypeInfo::StaticType(static_type::STREAM),
            Mutable::Generator(..) => TypeInfo::StaticType(static_type::GENERATOR),
            Mutable::GeneratorState(..) => TypeInfo::StaticType(static_type::GENERATOR_STATE),
            Mutable::Option(..) => TypeInfo::StaticType(static_type::OPTION),
            Mutable::Result(..) => TypeInfo::StaticType(static_type::RESULT),
            Mutable::Function(..) => TypeInfo::StaticType(static_type::FUNCTION),
            Mutable::Format(..) => TypeInfo::StaticType(static_type::FORMAT),
            Mutable::EmptyStruct(empty) => empty.type_info(),
            Mutable::TupleStruct(tuple) => tuple.type_info(),
            Mutable::Struct(object) => object.type_info(),
            Mutable::Variant(empty) => empty.type_info(),
            Mutable::Any(any) => any.type_info(),
        }
    }

    /// Get the type hash for the current value.
    ///
    /// One notable feature is that the type of a variant is its container
    /// *enum*, and not the type hash of the variant itself.
    pub(crate) fn type_hash(&self) -> Hash {
        match self {
            Mutable::String(..) => static_type::STRING.hash,
            Mutable::Bytes(..) => static_type::BYTES.hash,
            Mutable::Vec(..) => static_type::VEC.hash,
            Mutable::Tuple(..) => static_type::TUPLE.hash,
            Mutable::Object(..) => static_type::OBJECT.hash,
            Mutable::RangeFrom(..) => static_type::RANGE_FROM.hash,
            Mutable::RangeFull(..) => static_type::RANGE_FULL.hash,
            Mutable::RangeInclusive(..) => static_type::RANGE_INCLUSIVE.hash,
            Mutable::RangeToInclusive(..) => static_type::RANGE_TO_INCLUSIVE.hash,
            Mutable::RangeTo(..) => static_type::RANGE_TO.hash,
            Mutable::Range(..) => static_type::RANGE.hash,
            Mutable::ControlFlow(..) => static_type::CONTROL_FLOW.hash,
            Mutable::Future(..) => static_type::FUTURE.hash,
            Mutable::Stream(..) => static_type::STREAM.hash,
            Mutable::Generator(..) => static_type::GENERATOR.hash,
            Mutable::GeneratorState(..) => static_type::GENERATOR_STATE.hash,
            Mutable::Result(..) => static_type::RESULT.hash,
            Mutable::Option(..) => static_type::OPTION.hash,
            Mutable::Function(..) => static_type::FUNCTION.hash,
            Mutable::Format(..) => static_type::FORMAT.hash,
            Mutable::EmptyStruct(empty) => empty.rtti.hash,
            Mutable::TupleStruct(tuple) => tuple.rtti.hash,
            Mutable::Struct(object) => object.rtti.hash,
            Mutable::Variant(variant) => variant.rtti().enum_hash,
            Mutable::Any(any) => any.type_hash(),
        }
    }
}

/// Ensures that `Value` and `Repr` is niche-filled when used in common
/// combinations.
#[test]
fn size_of_value() {
    use core::mem::size_of;

    assert_eq!(size_of::<Repr>(), size_of::<Inline>());
    assert_eq!(size_of::<Repr>(), size_of::<Value>());
    assert_eq!(size_of::<Option<Value>>(), size_of::<Value>());
    assert_eq!(size_of::<Option<Repr>>(), size_of::<Repr>());
}
