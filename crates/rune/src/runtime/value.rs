#[macro_use]
mod macros;

mod inline;
pub use self::inline::Inline;

mod serde;

mod rtti;
pub use self::rtti::{Rtti, VariantRtti};

mod data;
pub use self::data::{EmptyStruct, Struct, TupleStruct};

use core::any;
use core::cmp::Ordering;
use core::fmt;
#[cfg(feature = "alloc")]
use core::hash::Hasher as _;
use core::mem::replace;
use core::ptr::NonNull;

use ::rust_alloc::sync::Arc;

use crate::alloc::fmt::TryWrite;
use crate::alloc::prelude::*;
use crate::alloc::{self, String};
use crate::compile::meta;
use crate::runtime;
use crate::{Any, Hash, TypeHash};

use super::static_type;
use super::{
    AccessError, AnyObj, AnyObjDrop, BorrowMut, BorrowRef, CallResultOnly, ConstValue,
    ConstValueKind, DynGuardedArgs, EnvProtocolCaller, Formatter, FromValue, Function, Future,
    Generator, IntoOutput, Iterator, MaybeTypeOf, Mut, Object, OwnedTuple, Protocol,
    ProtocolCaller, RawAnyObjGuard, Ref, RuntimeError, Shared, Snapshot, Stream, Type, TypeInfo,
    Variant, Vec, Vm, VmErrorKind, VmIntegerRepr, VmResult,
};
#[cfg(feature = "alloc")]
use super::{Hasher, Tuple};

/// Defined guard for a reference value.
///
/// See [Value::from_ref].
pub struct ValueRefGuard {
    #[allow(unused)]
    guard: AnyObjDrop,
}

/// Defined guard for a reference value.
///
/// See [Value::from_mut].
pub struct ValueMutGuard {
    #[allow(unused)]
    guard: AnyObjDrop,
}

/// The guard returned by [Value::into_any_mut_ptr].
pub struct RawValueGuard {
    #[allow(unused)]
    guard: RawAnyObjGuard,
}

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
    Any(AnyObj),
}

pub(crate) enum OwnedRepr {
    Inline(Inline),
    Mutable(Mutable),
    Any(AnyObj),
}

impl OwnedRepr {
    #[inline]
    pub(crate) fn type_info(&self) -> TypeInfo {
        match self {
            OwnedRepr::Inline(value) => value.type_info(),
            OwnedRepr::Mutable(value) => value.type_info(),
            OwnedRepr::Any(value) => value.type_info(),
        }
    }
}

pub(crate) enum RefRepr<'a> {
    Inline(&'a Inline),
    Mutable(&'a Shared<Mutable>),
    Any(&'a AnyObj),
}

impl RefRepr<'_> {
    #[inline]
    pub(crate) fn type_info(&self) -> Result<TypeInfo, AccessError> {
        match self {
            RefRepr::Inline(value) => Ok(value.type_info()),
            RefRepr::Mutable(value) => Ok(value.borrow_ref()?.type_info()),
            RefRepr::Any(value) => Ok(value.type_info()),
        }
    }
}

/// Access the internals of a value mutably.
pub(crate) enum MutRepr<'a> {
    Inline(&'a mut Inline),
    Mutable(#[allow(unused)] &'a mut Shared<Mutable>),
    Any(#[allow(unused)] &'a mut AnyObj),
}

pub(crate) enum BorrowRefRepr<'a> {
    Inline(&'a Inline),
    Mutable(BorrowRef<'a, Mutable>),
    Any(&'a AnyObj),
}

impl<'a> BorrowRefRepr<'a> {
    #[inline]
    pub(crate) fn type_info(&self) -> TypeInfo {
        match self {
            BorrowRefRepr::Inline(value) => value.type_info(),
            BorrowRefRepr::Mutable(value) => value.type_info(),
            BorrowRefRepr::Any(value) => value.type_info(),
        }
    }
}

pub(crate) enum ValueShared {
    Inline(Inline),
    Mutable(Shared<Mutable>),
    Any(AnyObj),
}

/// An entry on the stack.
pub struct Value {
    repr: Repr,
}

impl Value {
    /// Take a mutable value, replacing the original location with an empty value.
    #[inline]
    pub fn take(value: &mut Self) -> Self {
        replace(value, Self::empty())
    }

    /// Construct a value from a type that implements [`Any`] which owns the
    /// underlying value.
    pub fn new<T>(data: T) -> alloc::Result<Self>
    where
        T: Any,
    {
        Ok(Self {
            repr: Repr::Any(AnyObj::new(data)?),
        })
    }

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
    ///     let b = any.borrow_ref::<Foo>()?;
    ///     assert_eq!(b.0, 1u32);
    /// }
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    pub unsafe fn from_ref<T>(data: &T) -> alloc::Result<(Self, ValueRefGuard)>
    where
        T: Any,
    {
        let value = AnyObj::from_ref(data)?;
        let (value, guard) = AnyObj::into_drop_guard(value);

        let guard = ValueRefGuard { guard };

        Ok((
            Self {
                repr: Repr::Any(value),
            },
            guard,
        ))
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
    ///     let (any, guard) = Value::from_mut(&mut v)?;
    ///
    ///     if let Ok(mut v) = any.borrow_mut::<Foo>() {
    ///         v.0 += 1;
    ///     }
    ///
    ///     drop(guard);
    ///     assert!(any.borrow_mut::<Foo>().is_err());
    ///     drop(any);
    /// }
    ///
    /// assert_eq!(v.0, 2);
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    pub unsafe fn from_mut<T>(data: &mut T) -> alloc::Result<(Self, ValueMutGuard)>
    where
        T: Any,
    {
        let value = AnyObj::from_mut(data)?;
        let (value, guard) = AnyObj::into_drop_guard(value);

        let guard = ValueMutGuard { guard };

        Ok((
            Self {
                repr: Repr::Any(value),
            },
            guard,
        ))
    }

    /// Optionally get the snapshot of the value if available.
    pub(crate) fn snapshot(&self) -> Option<Snapshot> {
        match &self.repr {
            Repr::Mutable(value) => Some(value.snapshot()),
            Repr::Any(value) => Some(value.snapshot()),
            _ => None,
        }
    }

    /// Test if the value is writable.
    pub fn is_writable(&self) -> bool {
        match self.repr {
            Repr::Empty => false,
            Repr::Inline(..) => true,
            Repr::Mutable(ref value) => value.is_writable(),
            Repr::Any(ref any) => any.is_writable(),
        }
    }

    /// Test if the value is readable.
    pub fn is_readable(&self) -> bool {
        match &self.repr {
            Repr::Empty => false,
            Repr::Inline(..) => true,
            Repr::Mutable(ref value) => value.is_readable(),
            Repr::Any(ref any) => any.is_readable(),
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
            match vm_try!(self.borrow_ref_repr()) {
                BorrowRefRepr::Inline(value) => match value {
                    Inline::Char(c) => {
                        vm_try!(f.try_write_char(*c));
                    }
                    Inline::Unsigned(byte) => {
                        let mut buffer = itoa::Buffer::new();
                        vm_try!(f.try_write_str(buffer.format(*byte)));
                    }
                    Inline::Signed(integer) => {
                        let mut buffer = itoa::Buffer::new();
                        vm_try!(f.try_write_str(buffer.format(*integer)));
                    }
                    Inline::Float(float) => {
                        let mut buffer = ryu::Buffer::new();
                        vm_try!(f.try_write_str(buffer.format(*float)));
                    }
                    Inline::Bool(bool) => {
                        vm_try!(vm_write!(f, "{bool}"));
                    }
                    _ => {
                        break 'fallback;
                    }
                },
                _ => {
                    break 'fallback;
                }
            }

            return VmResult::Ok(());
        };

        let mut args = DynGuardedArgs::new((f,));

        let result =
            vm_try!(caller.call_protocol_fn(Protocol::STRING_DISPLAY, self.clone(), &mut args));

        VmResult::Ok(vm_try!(<()>::from_value(result)))
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
            let value = match vm_try!(self.as_ref_repr()) {
                RefRepr::Inline(value) => {
                    return VmResult::Ok(Self {
                        repr: Repr::Inline(*value),
                    });
                }
                RefRepr::Mutable(value) => match &*vm_try!(value.borrow_ref()) {
                    Mutable::Tuple(value) => Mutable::Tuple(vm_try!(value.try_clone())),
                    Mutable::Object(value) => Mutable::Object(vm_try!(value.try_clone())),
                    Mutable::Stream(value) => Mutable::Stream(vm_try!(value.try_clone())),
                    Mutable::Generator(value) => Mutable::Generator(vm_try!(value.try_clone())),
                    Mutable::Option(value) => Mutable::Option(vm_try!(value.try_clone())),
                    Mutable::Result(value) => Mutable::Result(vm_try!(value.try_clone())),
                    Mutable::EmptyStruct(value) => Mutable::EmptyStruct(vm_try!(value.try_clone())),
                    Mutable::TupleStruct(value) => Mutable::TupleStruct(vm_try!(value.try_clone())),
                    Mutable::Struct(value) => Mutable::Struct(vm_try!(value.try_clone())),
                    Mutable::Variant(value) => Mutable::Variant(vm_try!(value.try_clone())),
                    Mutable::Function(value) => Mutable::Function(vm_try!(value.try_clone())),
                    _ => {
                        break 'fallback;
                    }
                },
                RefRepr::Any(..) => {
                    break 'fallback;
                }
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
                    vm_try!(vm_write!(f, "<empty>"));
                    return VmResult::Ok(());
                }
                Repr::Inline(value) => {
                    vm_try!(vm_write!(f, "{value:?}"));
                    return VmResult::Ok(());
                }
                Repr::Mutable(ref value) => value,
                Repr::Any(..) => break 'fallback,
            };

            match &*vm_try!(value.borrow_ref()) {
                Mutable::Tuple(value) => {
                    vm_try!(vm_write!(f, "{value:?}"));
                }
                Mutable::Object(value) => {
                    vm_try!(vm_write!(f, "{value:?}"));
                }
                Mutable::Future(value) => {
                    vm_try!(vm_write!(f, "{value:?}"));
                }
                Mutable::Stream(value) => {
                    vm_try!(vm_write!(f, "{value:?}"));
                }
                Mutable::Generator(value) => {
                    vm_try!(vm_write!(f, "{value:?}"));
                }
                Mutable::Option(value) => {
                    vm_try!(vm_write!(f, "{value:?}"));
                }
                Mutable::Result(value) => {
                    vm_try!(vm_write!(f, "{value:?}"));
                }
                Mutable::EmptyStruct(value) => {
                    vm_try!(vm_write!(f, "{value:?}"));
                }
                Mutable::TupleStruct(value) => {
                    vm_try!(vm_write!(f, "{value:?}"));
                }
                Mutable::Struct(value) => {
                    vm_try!(vm_write!(f, "{value:?}"));
                }
                Mutable::Variant(value) => {
                    vm_try!(vm_write!(f, "{value:?}"));
                }
                Mutable::Function(value) => {
                    vm_try!(vm_write!(f, "{value:?}"));
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
                    vm_try!(vm_write!(f, "<empty>"));
                }
                Repr::Inline(value) => {
                    vm_try!(vm_write!(f, "{value:?}"));
                }
                Repr::Mutable(value) => {
                    let ty = vm_try!(value.borrow_ref()).type_info();
                    vm_try!(vm_write!(f, "<{ty} object at {value:p}>"));
                }
                Repr::Any(value) => {
                    let ty = value.type_info();
                    vm_try!(vm_write!(f, "<{ty} object at {value:p}>"));
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
                match name.as_kind() {
                    ConstValueKind::String(s) => {
                        return VmResult::Ok(vm_try!(String::try_from(s.as_str())))
                    }
                    _ => return err(VmErrorKind::expected::<String>(name.type_info())),
                }
            }

            if let Some(name) = unit.constant(hash) {
                match name.as_kind() {
                    ConstValueKind::String(s) => {
                        return VmResult::Ok(vm_try!(String::try_from(s.as_str())))
                    }
                    _ => return err(VmErrorKind::expected::<String>(name.type_info())),
                }
            }

            VmResult::Ok(vm_try!(vm_try!(self.type_info()).try_to_string()))
        })
    }

    /// Construct a vector.
    pub fn vec(vec: alloc::Vec<Value>) -> alloc::Result<Self> {
        let data = Vec::from(vec);
        Value::try_from(data)
    }

    /// Construct a tuple.
    pub fn tuple(vec: alloc::Vec<Value>) -> alloc::Result<Self> {
        let data = OwnedTuple::try_from(vec)?;
        Value::try_from(data)
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
        match self.repr {
            Repr::Mutable(value) => {
                drop(vm_try!(value.take()));
            }
            Repr::Any(value) => {
                vm_try!(value.drop());
            }
            _ => {}
        }

        VmResult::Ok(())
    }

    /// Move the interior value.
    pub(crate) fn move_(self) -> VmResult<Self> {
        match self.repr {
            Repr::Mutable(value) => VmResult::Ok(Value {
                repr: Repr::Mutable(vm_try!(Shared::new(vm_try!(value.take())))),
            }),
            Repr::Any(value) => VmResult::Ok(Value {
                repr: Repr::Any(vm_try!(value.take())),
            }),
            repr => VmResult::Ok(Value { repr }),
        }
    }

    /// Try to coerce value into a usize.
    #[inline]
    pub fn as_usize(&self) -> Result<usize, RuntimeError> {
        self.as_integer()
    }

    /// Get the value as a string.
    #[deprecated(
        note = "For consistency with other methods, this has been renamed Value::borrow_string_ref"
    )]
    #[inline]
    pub fn as_string(&self) -> Result<BorrowRef<'_, str>, RuntimeError> {
        self.borrow_string_ref()
    }

    /// Borrow the interior value as a string reference.
    pub fn borrow_string_ref(&self) -> Result<BorrowRef<'_, str>, RuntimeError> {
        let string = self.borrow_ref::<String>()?;
        Ok(BorrowRef::map(string, String::as_str))
    }

    /// Take the current value as a string.
    #[inline]
    pub fn into_string(self) -> Result<String, RuntimeError> {
        match self.take_repr()? {
            OwnedRepr::Any(value) => Ok(value.downcast()?),
            actual => Err(RuntimeError::expected::<String>(actual.type_info())),
        }
    }

    /// Coerce into type value.
    #[doc(hidden)]
    #[inline]
    pub fn into_type_value(self) -> Result<TypeValue, RuntimeError> {
        match self.take_repr()? {
            OwnedRepr::Inline(value) => match value {
                Inline::Unit => Ok(TypeValue::Unit),
                value => Ok(TypeValue::NotTypedInline(NotTypedInlineValue(value))),
            },
            OwnedRepr::Mutable(value) => match value {
                Mutable::Tuple(tuple) => Ok(TypeValue::Tuple(tuple)),
                Mutable::Object(object) => Ok(TypeValue::Object(object)),
                Mutable::EmptyStruct(empty) => Ok(TypeValue::EmptyStruct(empty)),
                Mutable::TupleStruct(tuple) => Ok(TypeValue::TupleStruct(tuple)),
                Mutable::Struct(object) => Ok(TypeValue::Struct(object)),
                Mutable::Variant(object) => Ok(TypeValue::Variant(object)),
                value => Ok(TypeValue::NotTypedMutable(NotTypedMutableValue(value))),
            },
            OwnedRepr::Any(value) => Ok(TypeValue::NotTypedRef(NotTypedRefValue(value))),
        }
    }

    /// Coerce into a unit.
    #[inline]
    pub fn into_unit(&self) -> Result<(), RuntimeError> {
        match self.borrow_ref_repr()? {
            BorrowRefRepr::Inline(Inline::Unit) => Ok(()),
            value => Err(RuntimeError::expected::<()>(value.type_info())),
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
        /// Coerce into [`char`].
        Char(char),
        as_char,
        as_char_mut,
    }

    inline_into! {
        /// Coerce into [`i64`] integer.
        Signed(i64),
        as_signed,
        as_signed_mut,
    }

    inline_into! {
        /// Coerce into [`u64`] unsigned integer.
        Unsigned(u64),
        as_unsigned,
        as_unsigned_mut,
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
        /// Coerce into a [`Function`].
        Function(Function),
        into_function_ref,
        into_function_mut,
        borrow_function_ref,
        borrow_function_mut,
        into_function,
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
        match self.repr {
            Repr::Empty => Err(RuntimeError::from(AccessError::empty())),
            Repr::Inline(value) => Err(RuntimeError::expected_any_obj(value.type_info())),
            Repr::Mutable(value) => Err(RuntimeError::expected_any_obj(
                value.borrow_ref()?.type_info(),
            )),
            Repr::Any(value) => Ok(value),
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
    pub fn into_future(self) -> Result<Future, RuntimeError> {
        let target = match self.take_repr()? {
            OwnedRepr::Mutable(Mutable::Future(future)) => return Ok(future),
            OwnedRepr::Inline(value) => Value::from(value),
            OwnedRepr::Mutable(value) => Value::try_from(value)?,
            OwnedRepr::Any(value) => Value::from(value),
        };

        let value = EnvProtocolCaller
            .call_protocol_fn(Protocol::INTO_FUTURE, target, &mut ())
            .into_result()?;

        Future::from_value(value)
    }

    /// Try to coerce value into a typed reference.
    #[inline]
    pub fn into_any_ref<T>(self) -> Result<Ref<T>, RuntimeError>
    where
        T: Any,
    {
        match self.repr {
            Repr::Empty => Err(RuntimeError::from(AccessError::empty())),
            Repr::Inline(value) => Err(RuntimeError::expected_any::<T>(value.type_info())),
            Repr::Mutable(value) => Err(RuntimeError::expected_any::<T>(
                value.borrow_ref()?.type_info(),
            )),
            Repr::Any(value) => Ok(value.into_ref()?),
        }
    }

    /// Try to coerce value into a typed reference.
    ///
    /// # Safety
    ///
    /// The returned pointer is only valid to dereference as long as the
    /// returned guard is live.
    #[inline]
    pub fn into_any_ref_ptr<T>(self) -> Result<(NonNull<T>, RawValueGuard), RuntimeError>
    where
        T: Any,
    {
        match self.repr {
            Repr::Empty => Err(RuntimeError::from(AccessError::empty())),
            Repr::Inline(value) => Err(RuntimeError::expected_any::<T>(value.type_info())),
            Repr::Mutable(value) => Err(RuntimeError::expected_any::<T>(
                value.borrow_ref()?.type_info(),
            )),
            Repr::Any(value) => {
                let (ptr, guard) = value.borrow_ref_ptr::<T>()?;
                let guard = RawValueGuard { guard };
                Ok((ptr, guard))
            }
        }
    }

    /// Try to coerce value into a typed mutable reference.
    #[inline]
    pub fn into_any_mut<T>(self) -> Result<Mut<T>, RuntimeError>
    where
        T: Any,
    {
        match self.repr {
            Repr::Empty => Err(RuntimeError::from(AccessError::empty())),
            Repr::Inline(value) => Err(RuntimeError::expected_any::<T>(value.type_info())),
            Repr::Mutable(value) => Err(RuntimeError::expected_any::<T>(
                value.borrow_ref()?.type_info(),
            )),
            Repr::Any(value) => Ok(value.into_mut()?),
        }
    }

    /// Try to coerce value into a typed mutable reference.
    ///
    /// # Safety
    ///
    /// The returned pointer is only valid to dereference as long as the
    /// returned guard is live.
    #[inline]
    pub fn into_any_mut_ptr<T>(self) -> Result<(NonNull<T>, RawValueGuard), RuntimeError>
    where
        T: Any,
    {
        match self.repr {
            Repr::Empty => Err(RuntimeError::from(AccessError::empty())),
            Repr::Inline(value) => Err(RuntimeError::expected_any::<T>(value.type_info())),
            Repr::Mutable(value) => Err(RuntimeError::expected_any::<T>(
                value.borrow_ref()?.type_info(),
            )),
            Repr::Any(value) => {
                let (ptr, guard) = value.borrow_mut_ptr::<T>()?;
                let guard = RawValueGuard { guard };
                Ok((ptr, guard))
            }
        }
    }

    /// Borrow the value as a typed reference.
    #[inline]
    pub fn borrow_ref<T>(&self) -> Result<BorrowRef<'_, T>, RuntimeError>
    where
        T: Any,
    {
        match &self.repr {
            Repr::Empty => Err(RuntimeError::from(AccessError::empty())),
            Repr::Inline(value) => Err(RuntimeError::expected_any::<T>(value.type_info())),
            Repr::Mutable(value) => Err(RuntimeError::expected_any::<T>(
                value.borrow_ref()?.type_info(),
            )),
            Repr::Any(value) => Ok(value.borrow_ref()?),
        }
    }

    /// Borrow the value as a mutable typed reference.
    #[inline]
    pub fn borrow_mut<T>(&self) -> Result<BorrowMut<'_, T>, RuntimeError>
    where
        T: Any,
    {
        match &self.repr {
            Repr::Empty => Err(RuntimeError::from(AccessError::empty())),
            Repr::Inline(value) => Err(RuntimeError::expected_any::<T>(value.type_info())),
            Repr::Mutable(value) => Err(RuntimeError::expected_any::<T>(
                value.borrow_ref()?.type_info(),
            )),
            Repr::Any(value) => Ok(value.borrow_mut()?),
        }
    }

    /// Try to coerce value into a typed value.
    #[inline]
    pub fn into_any<T>(self) -> Result<T, RuntimeError>
    where
        T: Any,
    {
        match self.repr {
            Repr::Empty => Err(RuntimeError::from(AccessError::empty())),
            Repr::Inline(value) => Err(RuntimeError::expected_any::<T>(value.type_info())),
            Repr::Mutable(value) => Err(RuntimeError::expected_any::<T>(
                value.borrow_ref()?.type_info(),
            )),
            Repr::Any(value) => Ok(value.downcast::<T>()?),
        }
    }

    /// Get the type hash for the current value.
    ///
    /// One notable feature is that the type of a variant is its container
    /// *enum*, and not the type hash of the variant itself.
    pub fn type_hash(&self) -> Result<Hash, RuntimeError> {
        match &self.repr {
            Repr::Empty => Err(RuntimeError::from(AccessError::empty())),
            Repr::Inline(value) => Ok(value.type_hash()),
            Repr::Mutable(value) => Ok(value.borrow_ref()?.type_hash()),
            Repr::Any(value) => Ok(value.type_hash()),
        }
    }

    /// Get the type information for the current value.
    pub fn type_info(&self) -> Result<TypeInfo, RuntimeError> {
        match &self.repr {
            Repr::Empty => Err(RuntimeError::from(AccessError::empty())),
            Repr::Inline(value) => Ok(value.type_info()),
            Repr::Mutable(value) => Ok(value.borrow_ref()?.type_info()),
            Repr::Any(value) => Ok(value.type_info()),
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
            let a = 'mutable: {
                match (vm_try!(self.as_ref_repr()), vm_try!(b.borrow_ref_repr())) {
                    (RefRepr::Inline(a), BorrowRefRepr::Inline(b)) => {
                        return VmResult::Ok(vm_try!(a.partial_eq(b)));
                    }
                    (RefRepr::Inline(a), b) => {
                        return err(VmErrorKind::UnsupportedBinaryOperation {
                            op: Protocol::PARTIAL_EQ.name,
                            lhs: a.type_info(),
                            rhs: b.type_info(),
                        });
                    }
                    (RefRepr::Mutable(a), BorrowRefRepr::Mutable(b)) => {
                        let a = vm_try!(a.borrow_ref());

                        match (&*a, &*b) {
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
                                    return Vec::eq_with(
                                        &a.data,
                                        &b.data,
                                        Value::partial_eq_with,
                                        caller,
                                    );
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
                            _ => {}
                        }

                        break 'mutable a;
                    }
                    (RefRepr::Any(value), _) => match value.type_hash() {
                        runtime::Vec::HASH => {
                            let vec = vm_try!(value.borrow_ref::<Vec>());
                            return Vec::partial_eq_with(&vec, b.clone(), caller);
                        }
                        _ => {
                            break 'fallback;
                        }
                    },
                    _ => {
                        break 'fallback;
                    }
                }
            };

            // Special cases.
            if let Mutable::Tuple(a) = &*a {
                return Vec::partial_eq_with(a, b.clone(), caller);
            }
        }

        if let CallResultOnly::Ok(value) = vm_try!(caller.try_call_protocol_fn(
            Protocol::PARTIAL_EQ,
            self.clone(),
            &mut Some((b.clone(),))
        )) {
            return VmResult::Ok(vm_try!(<_>::from_value(value)));
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
        match vm_try!(self.borrow_ref_repr()) {
            BorrowRefRepr::Inline(value) => match value {
                Inline::Unsigned(value) => {
                    hasher.write_u64(*value);
                    return VmResult::Ok(());
                }
                Inline::Signed(value) => {
                    hasher.write_i64(*value);
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
            BorrowRefRepr::Mutable(value) => {
                if let Mutable::Tuple(tuple) = &*value {
                    return Tuple::hash_with(tuple, hasher, caller);
                }
            }
            BorrowRefRepr::Any(value) if value.type_hash() == runtime::Vec::HASH => {
                let vec = vm_try!(value.borrow_ref::<Vec>());
                return Vec::hash_with(&vec, hasher, caller);
            }
            _ => {}
        }

        let mut args = DynGuardedArgs::new((hasher,));

        if let CallResultOnly::Ok(value) =
            vm_try!(caller.try_call_protocol_fn(Protocol::HASH, self.clone(), &mut args))
        {
            return VmResult::Ok(vm_try!(<_>::from_value(value)));
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
        match (
            vm_try!(self.borrow_ref_repr()),
            vm_try!(b.borrow_ref_repr()),
        ) {
            (BorrowRefRepr::Inline(a), BorrowRefRepr::Inline(b)) => {
                return a.eq(b);
            }
            (BorrowRefRepr::Inline(lhs), rhs) => {
                return err(VmErrorKind::UnsupportedBinaryOperation {
                    op: Protocol::EQ.name,
                    lhs: lhs.type_info(),
                    rhs: rhs.type_info(),
                });
            }
            (BorrowRefRepr::Mutable(a), BorrowRefRepr::Mutable(b)) => match (&*a, &*b) {
                (Mutable::Tuple(a), Mutable::Tuple(b)) => {
                    return Vec::eq_with(a, b, Value::eq_with, caller);
                }
                (Mutable::Object(a), Mutable::Object(b)) => {
                    return Object::eq_with(a, b, Value::eq_with, caller);
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
            return VmResult::Ok(vm_try!(<_>::from_value(value)));
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
        match (
            vm_try!(self.borrow_ref_repr()),
            vm_try!(b.borrow_ref_repr()),
        ) {
            (BorrowRefRepr::Inline(a), BorrowRefRepr::Inline(b)) => {
                return VmResult::Ok(vm_try!(a.partial_cmp(b)))
            }
            (BorrowRefRepr::Inline(lhs), rhs) => {
                return err(VmErrorKind::UnsupportedBinaryOperation {
                    op: Protocol::PARTIAL_CMP.name,
                    lhs: lhs.type_info(),
                    rhs: rhs.type_info(),
                })
            }
            (BorrowRefRepr::Mutable(a), BorrowRefRepr::Mutable(b)) => match (&*a, &*b) {
                (Mutable::Tuple(a), Mutable::Tuple(b)) => {
                    return Vec::partial_cmp_with(a, b, caller);
                }
                (Mutable::Object(a), Mutable::Object(b)) => {
                    return Object::partial_cmp_with(a, b, caller);
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
            return VmResult::Ok(vm_try!(<_>::from_value(value)));
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
        match (
            vm_try!(self.borrow_ref_repr()),
            vm_try!(b.borrow_ref_repr()),
        ) {
            (BorrowRefRepr::Inline(a), BorrowRefRepr::Inline(b)) => return a.cmp(b),
            (BorrowRefRepr::Mutable(a), BorrowRefRepr::Mutable(b)) => match (&*a, &*b) {
                (Mutable::Tuple(a), Mutable::Tuple(b)) => {
                    return Vec::cmp_with(a, b, caller);
                }
                (Mutable::Object(a), Mutable::Object(b)) => {
                    return Object::cmp_with(a, b, caller);
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
            (BorrowRefRepr::Inline(lhs), rhs) => {
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
            return VmResult::Ok(vm_try!(<_>::from_value(value)));
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
    /// let value = rune::to_value(u32::MAX)?;
    ///
    /// assert_eq!(value.as_integer::<u64>()?, u32::MAX as u64);
    /// assert!(value.as_integer::<i32>().is_err());
    ///
    /// # Ok::<(), rune::support::Error>(())
    /// ```
    pub fn as_integer<T>(&self) -> Result<T, RuntimeError>
    where
        T: TryFrom<u64> + TryFrom<i64>,
    {
        match self.repr {
            Repr::Empty => Err(RuntimeError::from(AccessError::empty())),
            Repr::Inline(value) => value.as_integer(),
            Repr::Mutable(ref value) => Err(RuntimeError::new(VmErrorKind::ExpectedNumber {
                actual: value.borrow_ref()?.type_info(),
            })),
            Repr::Any(ref value) => Err(RuntimeError::new(VmErrorKind::ExpectedNumber {
                actual: value.type_info(),
            })),
        }
    }

    pub(crate) fn as_inline_unchecked(&self) -> Option<&Inline> {
        match &self.repr {
            Repr::Inline(value) => Some(value),
            _ => None,
        }
    }

    /// Test if the value is inline.
    pub(crate) fn is_inline(&self) -> bool {
        matches!(self.repr, Repr::Inline(..))
    }

    /// Coerce into a checked [`Inline`] object.
    ///
    /// Any empty value will cause an access error.
    pub(crate) fn as_inline(&self) -> Result<Option<&Inline>, AccessError> {
        match &self.repr {
            Repr::Empty => Err(AccessError::empty()),
            Repr::Inline(value) => Ok(Some(value)),
            Repr::Mutable(..) => Ok(None),
            Repr::Any(..) => Ok(None),
        }
    }

    pub(crate) fn as_inline_mut(&mut self) -> Result<Option<&mut Inline>, AccessError> {
        match &mut self.repr {
            Repr::Empty => Err(AccessError::empty()),
            Repr::Inline(value) => Ok(Some(value)),
            Repr::Mutable(..) => Ok(None),
            Repr::Any(..) => Ok(None),
        }
    }

    /// Coerce into a checked [`AnyObj`] object.
    ///
    /// Any empty value will cause an access error.
    pub(crate) fn as_any(&self) -> Result<Option<&AnyObj>, AccessError> {
        match &self.repr {
            Repr::Empty => Err(AccessError::empty()),
            Repr::Inline(..) => Ok(None),
            Repr::Mutable(..) => Ok(None),
            Repr::Any(value) => Ok(Some(value)),
        }
    }

    pub(crate) fn take_repr(self) -> Result<OwnedRepr, AccessError> {
        match self.repr {
            Repr::Empty => Err(AccessError::empty()),
            Repr::Inline(value) => Ok(OwnedRepr::Inline(value)),
            Repr::Mutable(value) => Ok(OwnedRepr::Mutable(value.take()?)),
            Repr::Any(value) => Ok(OwnedRepr::Any(value)),
        }
    }

    pub(crate) fn borrow_ref_repr(&self) -> Result<BorrowRefRepr<'_>, AccessError> {
        match &self.repr {
            Repr::Empty => Err(AccessError::empty()),
            Repr::Inline(value) => Ok(BorrowRefRepr::Inline(value)),
            Repr::Mutable(value) => Ok(BorrowRefRepr::Mutable(value.borrow_ref()?)),
            Repr::Any(value) => Ok(BorrowRefRepr::Any(value)),
        }
    }

    pub(crate) fn as_ref_repr(&self) -> Result<RefRepr<'_>, AccessError> {
        match &self.repr {
            Repr::Inline(value) => Ok(RefRepr::Inline(value)),
            Repr::Mutable(value) => Ok(RefRepr::Mutable(value)),
            Repr::Any(value) => Ok(RefRepr::Any(value)),
            Repr::Empty => Err(AccessError::empty()),
        }
    }

    pub(crate) fn as_mut_repr(&mut self) -> Result<MutRepr<'_>, AccessError> {
        match &mut self.repr {
            Repr::Empty => Err(AccessError::empty()),
            Repr::Inline(value) => Ok(MutRepr::Inline(value)),
            Repr::Mutable(value) => Ok(MutRepr::Mutable(value)),
            Repr::Any(value) => Ok(MutRepr::Any(value)),
        }
    }

    pub(crate) fn try_borrow_ref<T>(&self) -> Result<Option<BorrowRef<'_, T>>, AccessError>
    where
        T: Any,
    {
        match &self.repr {
            Repr::Inline(..) => Ok(None),
            Repr::Mutable(..) => Ok(None),
            Repr::Any(value) => value.try_borrow_ref(),
            Repr::Empty => Err(AccessError::empty()),
        }
    }

    pub(crate) fn into_value_shared(self) -> Result<ValueShared, AccessError> {
        match self.repr {
            Repr::Empty => Err(AccessError::empty()),
            Repr::Inline(value) => Ok(ValueShared::Inline(value)),
            Repr::Mutable(value) => Ok(ValueShared::Mutable(value)),
            Repr::Any(value) => Ok(ValueShared::Any(value)),
        }
    }

    pub(crate) fn protocol_into_iter(&self) -> VmResult<Value> {
        EnvProtocolCaller.call_protocol_fn(Protocol::INTO_ITER, self.clone(), &mut ())
    }

    pub(crate) fn protocol_next(&self) -> VmResult<Option<Value>> {
        let value =
            vm_try!(EnvProtocolCaller.call_protocol_fn(Protocol::NEXT, self.clone(), &mut ()));

        VmResult::Ok(vm_try!(FromValue::from_value(value)))
    }

    pub(crate) fn protocol_next_back(&self) -> VmResult<Option<Value>> {
        let value =
            vm_try!(EnvProtocolCaller.call_protocol_fn(Protocol::NEXT_BACK, self.clone(), &mut ()));

        VmResult::Ok(vm_try!(FromValue::from_value(value)))
    }

    pub(crate) fn protocol_nth_back(&self, n: usize) -> VmResult<Option<Value>> {
        let value = vm_try!(EnvProtocolCaller.call_protocol_fn(
            Protocol::NTH_BACK,
            self.clone(),
            &mut Some((n,))
        ));

        VmResult::Ok(vm_try!(FromValue::from_value(value)))
    }

    pub(crate) fn protocol_len(&self) -> VmResult<usize> {
        let value =
            vm_try!(EnvProtocolCaller.call_protocol_fn(Protocol::LEN, self.clone(), &mut ()));

        VmResult::Ok(vm_try!(FromValue::from_value(value)))
    }

    pub(crate) fn protocol_size_hint(&self) -> VmResult<(usize, Option<usize>)> {
        let value =
            vm_try!(EnvProtocolCaller.call_protocol_fn(Protocol::SIZE_HINT, self.clone(), &mut ()));

        VmResult::Ok(vm_try!(FromValue::from_value(value)))
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
            Repr::Any(value) => value.snapshot(),
        };

        if !snapshot.is_readable() {
            write!(f, "<{snapshot}>")?;
            return Ok(());
        }

        let mut s = String::new();
        // SAFETY: Formatter does not outlive the string it references.
        let mut o = unsafe { Formatter::new(NonNull::from(&mut s)) };

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
                Repr::Any(value) => {
                    let ty = value.type_info();
                    write!(f, "<{ty} object at {value:p}: {e}>")?;
                }
            }

            return Ok(());
        }

        f.write_str(s.as_str())?;
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

impl From<AnyObj> for Value {
    #[inline]
    fn from(value: AnyObj) -> Self {
        Self {
            repr: Repr::Any(value),
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

impl TryFrom<&str> for Value {
    type Error = alloc::Error;

    #[inline]
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Value::new(String::try_from(value)?)
    }
}

impl IntoOutput for Mutable {
    type Output = Mutable;

    #[inline]
    fn into_output(self) -> VmResult<Self::Output> {
        VmResult::Ok(self)
    }
}

inline_from! {
    Bool => bool,
    Char => char,
    Signed => i64,
    Unsigned => u64,
    Float => f64,
    Type => Type,
    Ordering => Ordering,
}

from! {
    Function => Function,
    EmptyStruct => EmptyStruct,
    TupleStruct => TupleStruct,
    Struct => Struct,
    Variant => Variant,
    Object => Object,
    Tuple => OwnedTuple,
    Generator => Generator<Vm>,
    Future => Future,
    Stream => Stream<Vm>,
}

any_from! {
    crate::alloc::String,
    super::Bytes,
    super::Format,
    super::ControlFlow,
    super::GeneratorState,
    super::Vec,
}

from_container! {
    Option => Option<Value>,
    Result => Result<Value, Value>,
}

signed_value_trait!(i8, i16, i32, i128, isize);
unsigned_value_trait!(u8, u16, u32, u128, usize);
float_value_trait!(f32);

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
            Repr::Any(any) => Repr::Any(any.clone()),
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
            (Repr::Any(lhs), Repr::Any(rhs)) => {
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

/// Wrapper for an any ref value kind.
#[doc(hidden)]
pub struct NotTypedRefValue(AnyObj);

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
    /// Not a typed value.
    #[doc(hidden)]
    NotTypedRef(NotTypedRefValue),
}

impl TypeValue {
    /// Get the type info of the current value.
    #[doc(hidden)]
    pub fn type_info(&self) -> TypeInfo {
        match self {
            TypeValue::Unit => TypeInfo::static_type(static_type::TUPLE),
            TypeValue::Tuple(..) => TypeInfo::static_type(static_type::TUPLE),
            TypeValue::Object(..) => TypeInfo::static_type(static_type::OBJECT),
            TypeValue::EmptyStruct(empty) => empty.type_info(),
            TypeValue::TupleStruct(tuple) => tuple.type_info(),
            TypeValue::Struct(object) => object.type_info(),
            TypeValue::Variant(empty) => empty.type_info(),
            TypeValue::NotTypedInline(value) => value.0.type_info(),
            TypeValue::NotTypedMutable(value) => value.0.type_info(),
            TypeValue::NotTypedRef(value) => value.0.type_info(),
        }
    }
}

pub(crate) enum Mutable {
    /// A tuple.
    Tuple(OwnedTuple),
    /// An object.
    Object(Object),
    /// A stored future.
    Future(Future),
    /// A Stream.
    Stream(Stream<Vm>),
    /// A stored generator.
    Generator(Generator<Vm>),
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
}

impl Mutable {
    pub(crate) fn type_info(&self) -> TypeInfo {
        match self {
            Mutable::Tuple(..) => TypeInfo::static_type(static_type::TUPLE),
            Mutable::Object(..) => TypeInfo::static_type(static_type::OBJECT),
            Mutable::Future(..) => TypeInfo::static_type(static_type::FUTURE),
            Mutable::Stream(..) => TypeInfo::static_type(static_type::STREAM),
            Mutable::Generator(..) => TypeInfo::static_type(static_type::GENERATOR),
            Mutable::Option(..) => TypeInfo::static_type(static_type::OPTION),
            Mutable::Result(..) => TypeInfo::static_type(static_type::RESULT),
            Mutable::Function(..) => TypeInfo::static_type(static_type::FUNCTION),
            Mutable::EmptyStruct(empty) => empty.type_info(),
            Mutable::TupleStruct(tuple) => tuple.type_info(),
            Mutable::Struct(object) => object.type_info(),
            Mutable::Variant(empty) => empty.type_info(),
        }
    }

    /// Get the type hash for the current value.
    ///
    /// One notable feature is that the type of a variant is its container
    /// *enum*, and not the type hash of the variant itself.
    pub(crate) fn type_hash(&self) -> Hash {
        match self {
            Mutable::Tuple(..) => static_type::TUPLE.hash,
            Mutable::Object(..) => static_type::OBJECT.hash,
            Mutable::Future(..) => static_type::FUTURE.hash,
            Mutable::Stream(..) => static_type::STREAM.hash,
            Mutable::Generator(..) => static_type::GENERATOR.hash,
            Mutable::Result(..) => static_type::RESULT.hash,
            Mutable::Option(..) => static_type::OPTION.hash,
            Mutable::Function(..) => static_type::FUNCTION.hash,
            Mutable::EmptyStruct(empty) => empty.rtti.hash,
            Mutable::TupleStruct(tuple) => tuple.rtti.hash,
            Mutable::Struct(object) => object.rtti.hash,
            Mutable::Variant(variant) => variant.rtti().enum_hash,
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
