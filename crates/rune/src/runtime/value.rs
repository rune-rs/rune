#[macro_use]
mod macros;

#[cfg(test)]
mod tests;

mod inline;
pub use self::inline::Inline;

#[cfg(feature = "serde")]
mod serde;

mod rtti;
pub(crate) use self::rtti::RttiKind;
pub use self::rtti::{Accessor, Rtti};

mod data;
pub use self::data::{EmptyStruct, Struct, TupleStruct};

mod any_sequence;
pub use self::any_sequence::AnySequence;
pub(crate) use self::any_sequence::AnySequenceTakeError;

use core::any;
use core::cmp::Ordering;
use core::fmt;
use core::mem::replace;
use core::ptr::NonNull;

use rust_alloc::sync::Arc;

use crate::alloc::fmt::TryWrite;
use crate::alloc::prelude::*;
use crate::alloc::{self, String};
use crate::compile::meta;
use crate::{vm_try, Any, Hash, TypeHash};

use super::{
    AccessError, AnyObj, AnyObjDrop, BorrowMut, BorrowRef, CallResultOnly, ConstValue,
    ConstValueKind, DynGuardedArgs, EnvProtocolCaller, Formatter, FromValue, Future, Hasher,
    IntoOutput, Iterator, MaybeTypeOf, Mut, Object, OwnedTuple, Protocol, ProtocolCaller,
    RawAnyObjGuard, Ref, RuntimeError, Shared, Snapshot, Tuple, Type, TypeInfo, Vec, VmErrorKind,
    VmIntegerRepr, VmResult,
};

/// Defined guard for a reference value.
///
/// See [`Value::from_ref`].
pub struct ValueRefGuard {
    #[allow(unused)]
    guard: AnyObjDrop,
}

/// Defined guard for a reference value.
///
/// See [`Value::from_mut`].
pub struct ValueMutGuard {
    #[allow(unused)]
    guard: AnyObjDrop,
}

/// The guard returned by [`Value::into_any_mut_ptr`].
pub struct RawValueGuard {
    #[allow(unused)]
    guard: RawAnyObjGuard,
}

// Small helper function to build errors.
#[inline]
fn err<T, E>(error: E) -> VmResult<T>
where
    VmErrorKind: From<E>,
{
    VmResult::err(error)
}

#[derive(Clone)]
pub(crate) enum Repr {
    Inline(Inline),
    Dynamic(AnySequence<Arc<Rtti>, Value>),
    Any(AnyObj),
}

impl Repr {
    #[inline]
    pub(crate) fn type_info(&self) -> TypeInfo {
        match self {
            Repr::Inline(value) => value.type_info(),
            Repr::Dynamic(value) => value.type_info(),
            Repr::Any(value) => value.type_info(),
        }
    }
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
            Repr::Dynamic(value) => Some(value.snapshot()),
            Repr::Any(value) => Some(value.snapshot()),
            _ => None,
        }
    }

    /// Test if the value is writable.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::{Any, Value};
    ///
    /// #[derive(Any)]
    /// struct Struct(u32);
    ///
    /// let value = Value::new(Struct(42))?;
    ///
    /// {
    ///     assert!(value.is_writable());
    ///
    ///     let borrowed = value.borrow_mut::<Struct>()?;
    ///     assert!(!value.is_writable());
    ///     drop(borrowed);
    ///     assert!(value.is_writable());
    /// }
    ///
    /// let foo = Struct(42);
    ///
    /// {
    ///     let (value, guard) = unsafe { Value::from_ref(&foo)? };
    ///     assert!(value.is_readable());
    ///     assert!(!value.is_writable());
    /// }
    ///
    /// let mut foo = Struct(42);
    ///
    /// {
    ///     let (value, guard) = unsafe { Value::from_mut(&mut foo)? };
    ///     assert!(value.is_readable());
    ///     assert!(value.is_writable());
    /// }
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    pub fn is_writable(&self) -> bool {
        match self.repr {
            Repr::Inline(Inline::Empty) => false,
            Repr::Inline(..) => true,
            Repr::Dynamic(ref value) => value.is_writable(),
            Repr::Any(ref any) => any.is_writable(),
        }
    }

    /// Test if a value is readable.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::{Any, Value};
    ///
    /// #[derive(Any)]
    /// struct Struct(u32);
    ///
    /// let value = Value::new(Struct(42))?;
    ///
    /// {
    ///     assert!(value.is_writable());
    ///
    ///     let borrowed = value.borrow_mut::<Struct>()?;
    ///     assert!(!value.is_writable());
    ///     drop(borrowed);
    ///     assert!(value.is_writable());
    /// }
    ///
    /// let foo = Struct(42);
    ///
    /// {
    ///     let (value, guard) = unsafe { Value::from_ref(&foo)? };
    ///     assert!(value.is_readable());
    ///     assert!(!value.is_writable());
    /// }
    ///
    /// let mut foo = Struct(42);
    ///
    /// {
    ///     let (value, guard) = unsafe { Value::from_mut(&mut foo)? };
    ///     assert!(value.is_readable());
    ///     assert!(value.is_writable());
    /// }
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    pub fn is_readable(&self) -> bool {
        match &self.repr {
            Repr::Inline(Inline::Empty) => false,
            Repr::Inline(..) => true,
            Repr::Dynamic(ref value) => value.is_readable(),
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
        Self {
            repr: Repr::Inline(Inline::Empty),
        }
    }

    /// Format the value using the [`DISPLAY_FMT`] protocol.
    ///
    /// You must use [`Vm::with`] to specify which virtual machine this function
    /// is called inside.
    ///
    /// [`Vm::with`]: crate::Vm::with
    ///
    /// # Errors
    ///
    /// This function errors if called outside of a virtual machine.
    ///
    /// [`DISPLAY_FMT`]: Protocol::DISPLAY_FMT
    pub fn display_fmt(&self, f: &mut Formatter) -> VmResult<()> {
        self.display_fmt_with(f, &mut EnvProtocolCaller)
    }

    /// Internal impl of display_fmt with a customizable caller.
    #[cfg_attr(feature = "bench", inline(never))]
    pub(crate) fn display_fmt_with(
        &self,
        f: &mut Formatter,
        caller: &mut dyn ProtocolCaller,
    ) -> VmResult<()> {
        'fallback: {
            match self.as_ref() {
                Repr::Inline(value) => match value {
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
                        vm_try!(write!(f, "{bool}"));
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
            vm_try!(caller.call_protocol_fn(&Protocol::DISPLAY_FMT, self.clone(), &mut args));

        vm_try!(<()>::from_value(result));
        VmResult::Ok(())
    }

    /// Perform a shallow clone of the value using the [`CLONE`] protocol.
    ///
    /// You must use [`Vm::with`] to specify which virtual machine this function
    /// is called inside.
    ///
    /// [`Vm::with`]: crate::Vm::with
    ///
    /// # Errors
    ///
    /// This function errors if called outside of a virtual machine.
    ///
    /// [`CLONE`]: Protocol::CLONE
    pub fn clone_(&self) -> VmResult<Self> {
        self.clone_with(&mut EnvProtocolCaller)
    }

    pub(crate) fn clone_with(&self, caller: &mut dyn ProtocolCaller) -> VmResult<Value> {
        match self.as_ref() {
            Repr::Inline(value) => {
                return VmResult::Ok(Self {
                    repr: Repr::Inline(*value),
                });
            }
            Repr::Dynamic(value) => {
                // TODO: This type of cloning should be deep, not shallow.
                return VmResult::Ok(Self {
                    repr: Repr::Dynamic(value.clone()),
                });
            }
            Repr::Any(..) => {}
        }

        VmResult::Ok(vm_try!(caller.call_protocol_fn(
            &Protocol::CLONE,
            self.clone(),
            &mut ()
        )))
    }

    /// Debug format the value using the [`DEBUG_FMT`] protocol.
    ///
    /// You must use [`Vm::with`] to specify which virtual machine this function
    /// is called inside.
    ///
    /// [`Vm::with`]: crate::Vm::with
    ///
    /// # Errors
    ///
    /// This function errors if called outside of a virtual machine.
    ///
    /// [`DEBUG_FMT`]: Protocol::DEBUG_FMT
    pub fn debug_fmt(&self, f: &mut Formatter) -> VmResult<()> {
        self.debug_fmt_with(f, &mut EnvProtocolCaller)
    }

    /// Internal impl of debug_fmt with a customizable caller.
    pub(crate) fn debug_fmt_with(
        &self,
        f: &mut Formatter,
        caller: &mut dyn ProtocolCaller,
    ) -> VmResult<()> {
        match &self.repr {
            Repr::Inline(value) => {
                vm_try!(write!(f, "{value:?}"));
            }
            Repr::Dynamic(ref value) => {
                vm_try!(value.debug_fmt_with(f, caller));
            }
            Repr::Any(..) => {
                // reborrow f to avoid moving it
                let mut args = DynGuardedArgs::new((&mut *f,));

                match vm_try!(caller.try_call_protocol_fn(
                    &Protocol::DEBUG_FMT,
                    self.clone(),
                    &mut args
                )) {
                    CallResultOnly::Ok(value) => {
                        vm_try!(<()>::from_value(value));
                    }
                    CallResultOnly::Unsupported(value) => match &value.repr {
                        Repr::Inline(value) => {
                            vm_try!(write!(f, "{value:?}"));
                        }
                        Repr::Dynamic(value) => {
                            let ty = value.type_info();
                            vm_try!(write!(f, "<{ty} object at {value:p}>"));
                        }
                        Repr::Any(value) => {
                            let ty = value.type_info();
                            vm_try!(write!(f, "<{ty} object at {value:p}>"));
                        }
                    },
                }
            }
        }

        VmResult::Ok(())
    }

    /// Convert value into an iterator using the [`Protocol::INTO_ITER`]
    /// protocol.
    ///
    /// You must use [`Vm::with`] to specify which virtual machine this function
    /// is called inside.
    ///
    /// [`Vm::with`]: crate::Vm::with
    ///
    /// # Errors
    ///
    /// This function will error if called outside of a virtual machine context.
    pub fn into_iter(self) -> VmResult<Iterator> {
        self.into_iter_with(&mut EnvProtocolCaller)
    }

    pub(crate) fn into_iter_with(self, caller: &mut dyn ProtocolCaller) -> VmResult<Iterator> {
        let value = vm_try!(caller.call_protocol_fn(&Protocol::INTO_ITER, self, &mut ()));
        VmResult::Ok(Iterator::new(value))
    }

    /// Retrieves a human readable type name for the current value.
    ///
    /// You must use [`Vm::with`] to specify which virtual machine this function
    /// is called inside.
    ///
    /// [`Vm::with`]: crate::Vm::with
    ///
    /// # Errors
    ///
    /// This function errors in case the provided type cannot be converted into
    /// a name without the use of a [`Vm`] and one is not provided through the
    /// environment.
    ///
    /// [`Vm`]: crate::Vm
    pub fn into_type_name(self) -> VmResult<String> {
        let hash = Hash::associated_function(self.type_hash(), &Protocol::INTO_TYPE_NAME);

        crate::runtime::env::shared(|context, unit| {
            if let Some(name) = context.constant(&hash) {
                match name.as_kind() {
                    ConstValueKind::String(s) => {
                        return VmResult::Ok(vm_try!(String::try_from(s.as_str())))
                    }
                    _ => return err(VmErrorKind::expected::<String>(name.type_info())),
                }
            }

            if let Some(name) = unit.constant(&hash) {
                match name.as_kind() {
                    ConstValueKind::String(s) => {
                        return VmResult::Ok(vm_try!(String::try_from(s.as_str())))
                    }
                    _ => return err(VmErrorKind::expected::<String>(name.type_info())),
                }
            }

            VmResult::Ok(vm_try!(self.type_info().try_to_string()))
        })
    }

    /// Construct a vector.
    pub fn vec(vec: alloc::Vec<Value>) -> alloc::Result<Self> {
        let data = Vec::from(vec);
        Value::try_from(data)
    }

    /// Construct a tuple.
    pub fn tuple(vec: alloc::Vec<Value>) -> alloc::Result<Self> {
        Value::try_from(OwnedTuple::try_from(vec)?)
    }

    /// Construct an empty.
    pub fn empty_struct(rtti: Arc<Rtti>) -> alloc::Result<Self> {
        Ok(Value::from(AnySequence::new(rtti, [])?))
    }

    /// Construct a typed tuple.
    pub fn tuple_struct(
        rtti: Arc<Rtti>,
        data: impl IntoIterator<IntoIter: ExactSizeIterator, Item = Value>,
    ) -> alloc::Result<Self> {
        Ok(Value::from(AnySequence::new(rtti, data)?))
    }

    /// Drop the interior value.
    ///
    /// This consumes any live references of the value and accessing them in the
    /// future will result in an error.
    pub(crate) fn drop(self) -> VmResult<()> {
        match self.repr {
            Repr::Dynamic(value) => {
                vm_try!(value.drop());
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
            Repr::Dynamic(value) => VmResult::Ok(Value {
                repr: Repr::Dynamic(vm_try!(value.take())),
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
        match self.take_repr() {
            Repr::Any(value) => Ok(value.downcast()?),
            actual => Err(RuntimeError::expected::<String>(actual.type_info())),
        }
    }

    /// Coerce into type value.
    #[doc(hidden)]
    #[inline]
    pub fn as_type_value(&self) -> Result<TypeValue<'_>, RuntimeError> {
        match self.as_ref() {
            Repr::Inline(value) => match value {
                Inline::Unit => Ok(TypeValue::Unit),
                value => Ok(TypeValue::NotTypedInline(NotTypedInline(*value))),
            },
            Repr::Dynamic(value) => match value.rtti().kind {
                RttiKind::Empty => Ok(TypeValue::EmptyStruct(EmptyStruct { rtti: value.rtti() })),
                RttiKind::Tuple => Ok(TypeValue::TupleStruct(TupleStruct {
                    rtti: value.rtti(),
                    data: value.borrow_ref()?,
                })),
                RttiKind::Struct => Ok(TypeValue::Struct(Struct {
                    rtti: value.rtti(),
                    data: value.borrow_ref()?,
                })),
            },
            Repr::Any(value) => match value.type_hash() {
                OwnedTuple::HASH => Ok(TypeValue::Tuple(value.borrow_ref()?)),
                Object::HASH => Ok(TypeValue::Object(value.borrow_ref()?)),
                _ => Ok(TypeValue::NotTypedAnyObj(NotTypedAnyObj(value))),
            },
        }
    }

    /// Coerce into a unit.
    #[inline]
    pub fn into_unit(&self) -> Result<(), RuntimeError> {
        match self.as_ref() {
            Repr::Inline(Inline::Unit) => Ok(()),
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
        /// Coerce into [`Hash`][crate::Hash].
        Hash(Hash),
        as_hash,
        as_hash_mut,
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

    /// Borrow as a tuple.
    ///
    /// This ensures that the value has read access to the underlying value
    /// and does not consume it.
    #[inline]
    pub fn borrow_tuple_ref(&self) -> Result<BorrowRef<'_, Tuple>, RuntimeError> {
        match self.as_ref() {
            Repr::Inline(Inline::Unit) => Ok(BorrowRef::from_static(Tuple::new(&[]))),
            Repr::Inline(value) => Err(RuntimeError::expected::<Tuple>(value.type_info())),
            Repr::Dynamic(value) => Err(RuntimeError::expected::<Tuple>(value.type_info())),
            Repr::Any(value) => {
                let value = value.borrow_ref::<OwnedTuple>()?;
                let value = BorrowRef::map(value, OwnedTuple::as_ref);
                Ok(value)
            }
        }
    }

    /// Borrow as a tuple as mutable.
    ///
    /// This ensures that the value has write access to the underlying value and
    /// does not consume it.
    #[inline]
    pub fn borrow_tuple_mut(&self) -> Result<BorrowMut<'_, Tuple>, RuntimeError> {
        match self.as_ref() {
            Repr::Inline(Inline::Unit) => Ok(BorrowMut::from_ref(Tuple::new_mut(&mut []))),
            Repr::Inline(value) => Err(RuntimeError::expected::<Tuple>(value.type_info())),
            Repr::Dynamic(value) => Err(RuntimeError::expected::<Tuple>(value.type_info())),
            Repr::Any(value) => {
                let value = value.borrow_mut::<OwnedTuple>()?;
                let value = BorrowMut::map(value, OwnedTuple::as_mut);
                Ok(value)
            }
        }
    }

    /// Borrow as an owned tuple reference.
    ///
    /// This ensures that the value has read access to the underlying value and
    /// does not consume it.
    #[inline]
    pub fn into_tuple(&self) -> Result<Box<Tuple>, RuntimeError> {
        match self.as_ref() {
            Repr::Inline(Inline::Unit) => Ok(Tuple::from_boxed(Box::default())),
            Repr::Inline(value) => Err(RuntimeError::expected::<Tuple>(value.type_info())),
            Repr::Dynamic(value) => Err(RuntimeError::expected::<Tuple>(value.type_info())),
            Repr::Any(value) => Ok(value.clone().downcast::<OwnedTuple>()?.into_boxed_tuple()),
        }
    }

    /// Borrow as an owned tuple reference.
    ///
    /// This ensures that the value has read access to the underlying value and
    /// does not consume it.
    #[inline]
    pub fn into_tuple_ref(&self) -> Result<Ref<Tuple>, RuntimeError> {
        match self.as_ref() {
            Repr::Inline(Inline::Unit) => Ok(Ref::from_static(Tuple::new(&[]))),
            Repr::Inline(value) => Err(RuntimeError::expected::<Tuple>(value.type_info())),
            Repr::Dynamic(value) => Err(RuntimeError::expected::<Tuple>(value.type_info())),
            Repr::Any(value) => {
                let value = value.clone().into_ref::<OwnedTuple>()?;
                let value = Ref::map(value, OwnedTuple::as_ref);
                Ok(value)
            }
        }
    }

    /// Borrow as an owned tuple mutable.
    ///
    /// This ensures that the value has write access to the underlying value and
    /// does not consume it.
    #[inline]
    pub fn into_tuple_mut(&self) -> Result<Mut<Tuple>, RuntimeError> {
        match self.as_ref() {
            Repr::Inline(Inline::Unit) => Ok(Mut::from_static(Tuple::new_mut(&mut []))),
            Repr::Inline(value) => Err(RuntimeError::expected::<Tuple>(value.type_info())),
            Repr::Dynamic(value) => Err(RuntimeError::expected::<Tuple>(value.type_info())),
            Repr::Any(value) => {
                let value = value.clone().into_mut::<OwnedTuple>()?;
                let value = Mut::map(value, OwnedTuple::as_mut);
                Ok(value)
            }
        }
    }

    /// Coerce into an [`AnyObj`].
    #[inline]
    pub fn into_any_obj(self) -> Result<AnyObj, RuntimeError> {
        match self.repr {
            Repr::Inline(value) => Err(RuntimeError::expected_any_obj(value.type_info())),
            Repr::Dynamic(value) => Err(RuntimeError::expected_any_obj(value.type_info())),
            Repr::Any(value) => Ok(value),
        }
    }

    /// Coerce into a [`Shared<T>`].
    ///
    /// This type checks and coerces the value into a type which statically
    /// guarantees that the underlying type is of the given type.
    #[inline]
    pub fn into_shared<T>(self) -> Result<Shared<T>, RuntimeError>
    where
        T: Any,
    {
        match self.repr {
            Repr::Inline(value) => Err(RuntimeError::expected_any_obj(value.type_info())),
            Repr::Dynamic(value) => Err(RuntimeError::expected_any_obj(value.type_info())),
            Repr::Any(value) => Ok(value.into_shared()?),
        }
    }

    /// Coerce into a future, or convert into a future using the
    /// [Protocol::INTO_FUTURE] protocol.
    ///
    /// You must use [`Vm::with`] to specify which virtual machine this function
    /// is called inside.
    ///
    /// [`Vm::with`]: crate::Vm::with
    ///
    /// # Errors
    ///
    /// This function errors in case the provided type cannot be converted into
    /// a future without the use of a [`Vm`] and one is not provided through the
    /// environment.
    ///
    /// [`Vm`]: crate::Vm
    #[inline]
    pub fn into_future(self) -> Result<Future, RuntimeError> {
        let target = match self.repr {
            Repr::Any(value) => match value.type_hash() {
                Future::HASH => {
                    return Ok(value.downcast::<Future>()?);
                }
                _ => Value::from(value),
            },
            repr => Value::from(repr),
        };

        let value = EnvProtocolCaller
            .call_protocol_fn(&Protocol::INTO_FUTURE, target, &mut ())
            .into_result()?;

        Future::from_value(value)
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
            Repr::Inline(value) => Err(RuntimeError::expected_any::<T>(value.type_info())),
            Repr::Dynamic(value) => Err(RuntimeError::expected_any::<T>(value.type_info())),
            Repr::Any(value) => {
                let (ptr, guard) = value.borrow_ref_ptr::<T>()?;
                let guard = RawValueGuard { guard };
                Ok((ptr, guard))
            }
        }
    }

    /// Try to coerce value into a typed mutable reference.
    ///
    /// # Safety
    ///
    /// The returned pointer is only valid to dereference as long as the
    /// returned guard is live.
    #[inline]
    #[doc(hidden)]
    pub fn into_any_mut_ptr<T>(self) -> Result<(NonNull<T>, RawValueGuard), RuntimeError>
    where
        T: Any,
    {
        match self.repr {
            Repr::Inline(value) => Err(RuntimeError::expected_any::<T>(value.type_info())),
            Repr::Dynamic(value) => Err(RuntimeError::expected_any::<T>(value.type_info())),
            Repr::Any(value) => {
                let (ptr, guard) = value.borrow_mut_ptr::<T>()?;
                let guard = RawValueGuard { guard };
                Ok((ptr, guard))
            }
        }
    }

    /// Downcast the value into a stored value that implements `Any`.
    ///
    /// This takes the interior value, making it inaccessible to other owned
    /// references.
    ///
    /// You should usually prefer to use [`rune::from_value`] instead of this
    /// directly.
    ///
    /// [`rune::from_value`]: crate::from_value
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::Value;
    /// use rune::alloc::String;
    ///
    /// let a = Value::try_from("Hello World")?;
    /// let b = a.clone();
    ///
    /// assert!(b.borrow_ref::<String>().is_ok());
    ///
    /// // NB: The interior representation of the stored string is from rune-alloc.
    /// let a = a.downcast::<String>()?;
    ///
    /// assert!(b.borrow_ref::<String>().is_err());
    ///
    /// assert_eq!(a, "Hello World");
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    #[inline]
    pub fn downcast<T>(self) -> Result<T, RuntimeError>
    where
        T: Any,
    {
        match self.repr {
            Repr::Inline(value) => Err(RuntimeError::expected_any::<T>(value.type_info())),
            Repr::Dynamic(value) => Err(RuntimeError::expected_any::<T>(value.type_info())),
            Repr::Any(value) => Ok(value.downcast::<T>()?),
        }
    }

    /// Borrow the value as a typed reference of type `T`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::Value;
    /// use rune::alloc::String;
    ///
    /// let a = Value::try_from("Hello World")?;
    /// let b = a.clone();
    ///
    /// assert!(b.borrow_ref::<String>().is_ok());
    ///
    /// // NB: The interior representation of the stored string is from rune-alloc.
    /// let a = a.downcast::<String>()?;
    ///
    /// assert!(b.borrow_ref::<String>().is_err());
    ///
    /// assert_eq!(a, "Hello World");
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    #[inline]
    pub fn borrow_ref<T>(&self) -> Result<BorrowRef<'_, T>, RuntimeError>
    where
        T: Any,
    {
        match &self.repr {
            Repr::Inline(value) => Err(RuntimeError::expected_any::<T>(value.type_info())),
            Repr::Dynamic(value) => Err(RuntimeError::expected_any::<T>(value.type_info())),
            Repr::Any(value) => Ok(value.borrow_ref()?),
        }
    }

    /// Try to coerce value into a typed reference of type `T`.
    ///
    /// You should usually prefer to use [`rune::from_value`] instead of this
    /// directly.
    ///
    /// [`rune::from_value`]: crate::from_value
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::Value;
    /// use rune::alloc::String;
    ///
    /// let mut a = Value::try_from("Hello World")?;
    /// let b = a.clone();
    ///
    /// assert_eq!(a.into_ref::<String>()?.as_str(), "Hello World");
    /// assert_eq!(b.into_ref::<String>()?.as_str(), "Hello World");
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    #[inline]
    pub fn into_ref<T>(self) -> Result<Ref<T>, RuntimeError>
    where
        T: Any,
    {
        match self.repr {
            Repr::Inline(value) => Err(RuntimeError::expected_any::<T>(value.type_info())),
            Repr::Dynamic(value) => Err(RuntimeError::expected_any::<T>(value.type_info())),
            Repr::Any(value) => Ok(value.into_ref()?),
        }
    }

    /// Try to borrow value into a typed mutable reference of type `T`.
    #[inline]
    pub fn borrow_mut<T>(&self) -> Result<BorrowMut<'_, T>, RuntimeError>
    where
        T: Any,
    {
        match &self.repr {
            Repr::Inline(value) => Err(RuntimeError::expected_any::<T>(value.type_info())),
            Repr::Dynamic(value) => Err(RuntimeError::expected_any::<T>(value.type_info())),
            Repr::Any(value) => Ok(value.borrow_mut()?),
        }
    }

    /// Try to coerce value into a typed mutable reference of type `T`.
    ///
    /// You should usually prefer to use [`rune::from_value`] instead of this
    /// directly since it supports transparently coercing into types like
    /// [`Mut<str>`].
    ///
    /// [`rune::from_value`]: crate::from_value
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::{Mut, Value};
    /// use rune::alloc::String;
    ///
    /// let mut a = Value::try_from("Hello World")?;
    /// let b = a.clone();
    ///
    /// fn modify_string(mut s: Mut<String>) {
    ///     assert_eq!(s.as_str(), "Hello World");
    ///     s.make_ascii_lowercase();
    ///     assert_eq!(s.as_str(), "hello world");
    /// }
    ///
    /// modify_string(a.into_mut::<String>()?);
    ///
    /// assert_eq!(b.borrow_mut::<String>()?.as_str(), "hello world");
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    #[inline]
    pub fn into_mut<T>(self) -> Result<Mut<T>, RuntimeError>
    where
        T: Any,
    {
        match self.repr {
            Repr::Inline(value) => Err(RuntimeError::expected_any::<T>(value.type_info())),
            Repr::Dynamic(value) => Err(RuntimeError::expected_any::<T>(value.type_info())),
            Repr::Any(value) => Ok(value.into_mut()?),
        }
    }

    /// Get the type hash for the current value.
    ///
    /// One notable feature is that the type of a variant is its container
    /// *enum*, and not the type hash of the variant itself.
    #[inline(always)]
    pub fn type_hash(&self) -> Hash {
        match &self.repr {
            Repr::Inline(value) => value.type_hash(),
            Repr::Dynamic(value) => value.type_hash(),
            Repr::Any(value) => value.type_hash(),
        }
    }

    /// Get the type information for the current value.
    #[inline(always)]
    pub fn type_info(&self) -> TypeInfo {
        match &self.repr {
            Repr::Inline(value) => value.type_info(),
            Repr::Dynamic(value) => value.type_info(),
            Repr::Any(value) => value.type_info(),
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
        self.bin_op_with(
            b,
            caller,
            &Protocol::PARTIAL_EQ,
            Inline::partial_eq,
            |lhs, rhs, caller| {
                if lhs.0.variant_hash != rhs.0.variant_hash {
                    return VmResult::Ok(false);
                }

                Vec::eq_with(lhs.1, rhs.1, Value::partial_eq_with, caller)
            },
        )
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
        self.bin_op_with(b, caller, &Protocol::EQ, Inline::eq, |lhs, rhs, caller| {
            if lhs.0.variant_hash != rhs.0.variant_hash {
                return VmResult::Ok(false);
            }

            Vec::eq_with(lhs.1, rhs.1, Value::eq_with, caller)
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
        self.bin_op_with(
            b,
            caller,
            &Protocol::PARTIAL_CMP,
            Inline::partial_cmp,
            |lhs, rhs, caller| {
                let ord = lhs.0.variant_hash.cmp(&rhs.0.variant_hash);

                if ord != Ordering::Equal {
                    return VmResult::Ok(Some(ord));
                }

                Vec::partial_cmp_with(lhs.1, rhs.1, caller)
            },
        )
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
        self.bin_op_with(
            b,
            caller,
            &Protocol::CMP,
            Inline::cmp,
            |lhs, rhs, caller| {
                let ord = lhs.0.variant_hash.cmp(&rhs.0.variant_hash);

                if ord != Ordering::Equal {
                    return VmResult::Ok(ord);
                }

                Vec::cmp_with(lhs.1, rhs.1, caller)
            },
        )
    }

    /// Hash the current value.
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
        match self.as_ref() {
            Repr::Inline(value) => {
                vm_try!(value.hash(hasher));
                return VmResult::Ok(());
            }
            Repr::Any(value) => match value.type_hash() {
                Vec::HASH => {
                    let vec = vm_try!(value.borrow_ref::<Vec>());
                    return Vec::hash_with(&vec, hasher, caller);
                }
                OwnedTuple::HASH => {
                    let tuple = vm_try!(value.borrow_ref::<OwnedTuple>());
                    return Tuple::hash_with(&tuple, hasher, caller);
                }
                _ => {}
            },
            _ => {}
        }

        let mut args = DynGuardedArgs::new((hasher,));

        if let CallResultOnly::Ok(value) =
            vm_try!(caller.try_call_protocol_fn(&Protocol::HASH, self.clone(), &mut args))
        {
            vm_try!(<()>::from_value(value));
            return VmResult::Ok(());
        }

        err(VmErrorKind::UnsupportedUnaryOperation {
            op: Protocol::HASH.name,
            operand: self.type_info(),
        })
    }

    fn bin_op_with<T>(
        &self,
        b: &Value,
        caller: &mut dyn ProtocolCaller,
        protocol: &'static Protocol,
        inline: fn(&Inline, &Inline) -> Result<T, RuntimeError>,
        dynamic: fn(
            (&Arc<Rtti>, &[Value]),
            (&Arc<Rtti>, &[Value]),
            &mut dyn ProtocolCaller,
        ) -> VmResult<T>,
    ) -> VmResult<T>
    where
        T: FromValue,
    {
        match (self.as_ref(), b.as_ref()) {
            (Repr::Inline(lhs), Repr::Inline(rhs)) => {
                return VmResult::Ok(vm_try!(inline(lhs, rhs)))
            }
            (Repr::Inline(lhs), rhs) => {
                return VmResult::err(VmErrorKind::UnsupportedBinaryOperation {
                    op: protocol.name,
                    lhs: lhs.type_info(),
                    rhs: rhs.type_info(),
                });
            }
            (Repr::Dynamic(lhs), Repr::Dynamic(rhs)) => {
                let lhs_rtti = lhs.rtti();
                let rhs_rtti = rhs.rtti();

                let lhs = vm_try!(lhs.borrow_ref());
                let rhs = vm_try!(rhs.borrow_ref());

                if lhs_rtti.hash == rhs_rtti.hash {
                    return dynamic((lhs_rtti, &lhs), (rhs_rtti, &rhs), caller);
                }

                return VmResult::err(VmErrorKind::UnsupportedBinaryOperation {
                    op: protocol.name,
                    lhs: lhs_rtti.clone().type_info(),
                    rhs: rhs_rtti.clone().type_info(),
                });
            }
            _ => {}
        }

        if let CallResultOnly::Ok(value) =
            vm_try!(caller.try_call_protocol_fn(protocol, self.clone(), &mut Some((b.clone(),))))
        {
            return VmResult::Ok(vm_try!(T::from_value(value)));
        }

        err(VmErrorKind::UnsupportedBinaryOperation {
            op: protocol.name,
            lhs: self.type_info(),
            rhs: b.type_info(),
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
            Repr::Inline(value) => value.as_integer(),
            Repr::Dynamic(ref value) => Err(RuntimeError::new(VmErrorKind::ExpectedNumber {
                actual: value.type_info(),
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
    #[inline]
    pub(crate) fn as_inline(&self) -> Option<&Inline> {
        match &self.repr {
            Repr::Inline(value) => Some(value),
            Repr::Dynamic(..) => None,
            Repr::Any(..) => None,
        }
    }

    #[inline]
    pub(crate) fn as_inline_mut(&mut self) -> Option<&mut Inline> {
        match &mut self.repr {
            Repr::Inline(value) => Some(value),
            Repr::Dynamic(..) => None,
            Repr::Any(..) => None,
        }
    }

    /// Coerce into a checked [`AnyObj`] object.
    ///
    /// Any empty value will cause an access error.
    #[inline]
    pub fn as_any(&self) -> Option<&AnyObj> {
        match &self.repr {
            Repr::Inline(..) => None,
            Repr::Dynamic(..) => None,
            Repr::Any(value) => Some(value),
        }
    }

    #[inline(always)]
    pub(crate) fn take_repr(self) -> Repr {
        self.repr
    }

    #[inline(always)]
    pub(crate) fn as_ref(&self) -> &Repr {
        &self.repr
    }

    #[inline(always)]
    pub(crate) fn as_mut(&mut self) -> &mut Repr {
        &mut self.repr
    }

    #[inline]
    pub(crate) fn try_borrow_ref<T>(&self) -> Result<Option<BorrowRef<'_, T>>, AccessError>
    where
        T: Any,
    {
        match &self.repr {
            Repr::Inline(..) => Ok(None),
            Repr::Dynamic(..) => Ok(None),
            Repr::Any(value) => value.try_borrow_ref(),
        }
    }

    #[inline]
    pub(crate) fn try_borrow_mut<T>(&self) -> Result<Option<BorrowMut<'_, T>>, AccessError>
    where
        T: Any,
    {
        match &self.repr {
            Repr::Inline(..) => Ok(None),
            Repr::Dynamic(..) => Ok(None),
            Repr::Any(value) => value.try_borrow_mut(),
        }
    }

    pub(crate) fn protocol_into_iter(&self) -> VmResult<Value> {
        EnvProtocolCaller.call_protocol_fn(&Protocol::INTO_ITER, self.clone(), &mut ())
    }

    pub(crate) fn protocol_next(&self) -> VmResult<Option<Value>> {
        let value =
            vm_try!(EnvProtocolCaller.call_protocol_fn(&Protocol::NEXT, self.clone(), &mut ()));

        VmResult::Ok(vm_try!(FromValue::from_value(value)))
    }

    pub(crate) fn protocol_next_back(&self) -> VmResult<Option<Value>> {
        let value = vm_try!(EnvProtocolCaller.call_protocol_fn(
            &Protocol::NEXT_BACK,
            self.clone(),
            &mut ()
        ));

        VmResult::Ok(vm_try!(FromValue::from_value(value)))
    }

    pub(crate) fn protocol_nth_back(&self, n: usize) -> VmResult<Option<Value>> {
        let value = vm_try!(EnvProtocolCaller.call_protocol_fn(
            &Protocol::NTH_BACK,
            self.clone(),
            &mut Some((n,))
        ));

        VmResult::Ok(vm_try!(FromValue::from_value(value)))
    }

    pub(crate) fn protocol_len(&self) -> VmResult<usize> {
        let value =
            vm_try!(EnvProtocolCaller.call_protocol_fn(&Protocol::LEN, self.clone(), &mut ()));

        VmResult::Ok(vm_try!(FromValue::from_value(value)))
    }

    pub(crate) fn protocol_size_hint(&self) -> VmResult<(usize, Option<usize>)> {
        let value = vm_try!(EnvProtocolCaller.call_protocol_fn(
            &Protocol::SIZE_HINT,
            self.clone(),
            &mut ()
        ));

        VmResult::Ok(vm_try!(FromValue::from_value(value)))
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.repr {
            Repr::Inline(value) => {
                write!(f, "{value:?}")?;
            }
            _ => {
                let mut s = String::new();
                let result = Formatter::format_with(&mut s, |f| self.debug_fmt(f));

                if let Err(e) = result.into_result() {
                    match &self.repr {
                        Repr::Inline(value) => {
                            write!(f, "<{value:?}: {e}>")?;
                        }
                        Repr::Dynamic(value) => {
                            let ty = value.type_info();
                            write!(f, "<{ty} object at {value:p}: {e}>")?;
                        }
                        Repr::Any(value) => {
                            let ty = value.type_info();
                            write!(f, "<{ty} object at {value:p}: {e}>")?;
                        }
                    }

                    return Ok(());
                }

                f.write_str(s.as_str())?;
            }
        }

        Ok(())
    }
}

impl From<Repr> for Value {
    #[inline]
    fn from(repr: Repr) -> Self {
        Self { repr }
    }
}

impl From<()> for Value {
    #[inline]
    fn from((): ()) -> Self {
        Value::from(Inline::Unit)
    }
}

impl IntoOutput for () {
    #[inline]
    fn into_output(self) -> Result<Value, RuntimeError> {
        Ok(Value::from(()))
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

/// Conversion from a [`AnyObj`] into a [`Value`].
///
/// # Examples
///
/// ```
/// use rune::Value;
/// use rune::runtime::AnyObj;
/// use rune::alloc::String;
///
/// let string = String::try_from("Hello World")?;
/// let string = AnyObj::new(string)?;
/// let string = Value::from(string);
///
/// let string = string.into_shared::<String>()?;
/// assert_eq!(string.borrow_ref()?.as_str(), "Hello World");
/// # Ok::<_, rune::support::Error>(())
/// ```
impl From<AnyObj> for Value {
    #[inline]
    fn from(value: AnyObj) -> Self {
        Self {
            repr: Repr::Any(value),
        }
    }
}

/// Conversion from a [`Shared<T>`] into a [`Value`].
///
/// # Examples
///
/// ```
/// use rune::Value;
/// use rune::runtime::Shared;
/// use rune::alloc::String;
///
/// let string = String::try_from("Hello World")?;
/// let string = Shared::new(string)?;
/// let string = Value::from(string);
///
/// let string = string.into_any_obj()?;
/// assert_eq!(string.borrow_ref::<String>()?.as_str(), "Hello World");
/// # Ok::<_, rune::support::Error>(())
/// ```
impl<T> From<Shared<T>> for Value
where
    T: Any,
{
    #[inline]
    fn from(value: Shared<T>) -> Self {
        Self {
            repr: Repr::Any(value.into_any_obj()),
        }
    }
}

impl IntoOutput for Inline {
    #[inline]
    fn into_output(self) -> Result<Value, RuntimeError> {
        Ok(Value::from(self))
    }
}

impl From<AnySequence<Arc<Rtti>, Value>> for Value {
    #[inline]
    fn from(value: AnySequence<Arc<Rtti>, Value>) -> Self {
        Self {
            repr: Repr::Dynamic(value),
        }
    }
}

impl TryFrom<&str> for Value {
    type Error = alloc::Error;

    #[inline]
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Value::new(String::try_from(value)?)
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
    Hash => Hash,
}

any_from! {
    crate::alloc::String,
    super::Bytes,
    super::Format,
    super::ControlFlow,
    super::GeneratorState,
    super::Vec,
    super::OwnedTuple,
    super::Generator,
    super::Stream,
    super::Function,
    super::Future,
    super::Object,
    Option<Value>,
    Result<Value, Value>,
}

signed_value_from!(i8, i16, i32);
signed_value_try_from!(i128, isize);
unsigned_value_from!(u8, u16, u32);
unsigned_value_try_from!(u128, usize);
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
            Repr::Inline(inline) => Repr::Inline(*inline),
            Repr::Dynamic(mutable) => Repr::Dynamic(mutable.clone()),
            Repr::Any(any) => Repr::Any(any.clone()),
        };

        Self { repr }
    }

    #[inline]
    fn clone_from(&mut self, source: &Self) {
        match (&mut self.repr, &source.repr) {
            (Repr::Inline(lhs), Repr::Inline(rhs)) => {
                *lhs = *rhs;
            }
            (Repr::Dynamic(lhs), Repr::Dynamic(rhs)) => {
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
pub struct NotTypedInline(Inline);

/// Wrapper for an any ref value kind.
#[doc(hidden)]
pub struct NotTypedAnyObj<'a>(&'a AnyObj);

/// The coersion of a value into a typed value.
#[non_exhaustive]
#[doc(hidden)]
pub enum TypeValue<'a> {
    /// The unit value.
    Unit,
    /// A tuple.
    Tuple(BorrowRef<'a, OwnedTuple>),
    /// An object.
    Object(BorrowRef<'a, Object>),
    /// An struct with a well-defined type.
    EmptyStruct(EmptyStruct<'a>),
    /// A tuple with a well-defined type.
    TupleStruct(TupleStruct<'a>),
    /// An struct with a well-defined type.
    Struct(Struct<'a>),
    /// Not a typed immutable value.
    #[doc(hidden)]
    NotTypedInline(NotTypedInline),
    /// Not a typed value.
    #[doc(hidden)]
    NotTypedAnyObj(NotTypedAnyObj<'a>),
}

impl TypeValue<'_> {
    /// Get the type info of the current value.
    #[doc(hidden)]
    pub fn type_info(&self) -> TypeInfo {
        match self {
            TypeValue::Unit => TypeInfo::any::<OwnedTuple>(),
            TypeValue::Tuple(..) => TypeInfo::any::<OwnedTuple>(),
            TypeValue::Object(..) => TypeInfo::any::<Object>(),
            TypeValue::EmptyStruct(empty) => empty.type_info(),
            TypeValue::TupleStruct(tuple) => tuple.type_info(),
            TypeValue::Struct(object) => object.type_info(),
            TypeValue::NotTypedInline(value) => value.0.type_info(),
            TypeValue::NotTypedAnyObj(value) => value.0.type_info(),
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
