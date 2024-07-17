mod serde;

use core::any;
use core::borrow::Borrow;
use core::cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd};
use core::fmt;
use core::hash;

use ::rust_alloc::sync::Arc;

use crate as rune;
use crate::alloc::fmt::TryWrite;
use crate::alloc::prelude::*;
use crate::alloc::{self, String};
use crate::compile::ItemBuf;
use crate::runtime::vm::CallResultOnly;
use crate::runtime::{
    AccessError, AccessErrorKind, AnyObj, AnyObjError, BorrowMut, BorrowRef, Bytes, ConstValue,
    ControlFlow, EnvProtocolCaller, Format, Formatter, FromValue, FullTypeOf, Function, Future,
    Generator, GeneratorState, IntoOutput, Iterator, MaybeTypeOf, Mut, Object, OwnedTuple,
    Protocol, ProtocolCaller, Range, RangeFrom, RangeFull, RangeInclusive, RangeTo,
    RangeToInclusive, Ref, RuntimeError, Shared, SharedPointerGuard, Snapshot, Stream, ToValue,
    Type, TypeInfo, Variant, Vec, Vm, VmErrorKind, VmIntegerRepr, VmResult,
};
#[cfg(feature = "alloc")]
use crate::runtime::{Hasher, Tuple};
use crate::{Any, Hash};

use ::serde::{Deserialize, Serialize};

/// Macro used to generate coersions for [`Value`].
macro_rules! into_base {
    (
        $(#[$($meta:meta)*])*
        $kind:ident($ty:ty),
        $into_ref:ident,
        $into_mut:ident,
        $borrow_ref:ident,
        $borrow_mut:ident,
    ) => {
        $(#[$($meta)*])*
        ///
        /// This ensures that the value has read access to the underlying value
        /// and does not consume it.
        #[inline]
        pub fn $into_ref(self) -> Result<Ref<$ty>, RuntimeError> {
            let result = Ref::try_map(self.into_kind_ref()?, |kind| match kind {
                ValueKind::$kind(bytes) => Some(bytes),
                _ => None,
            });

            match result {
                Ok(bytes) => Ok(bytes),
                Err(actual) => {
                    Err(RuntimeError::expected::<$ty>(actual.type_info()))
                }
            }
        }

        $(#[$($meta)*])*
        ///
        /// This ensures that the value has write access to the underlying value
        /// and does not consume it.
        #[inline]
        pub fn $into_mut(self) -> Result<Mut<$ty>, RuntimeError> {
            let result = Mut::try_map(self.into_kind_mut()?, |kind| match kind {
                ValueKind::$kind(bytes) => Some(bytes),
                _ => None,
            });

            match result {
                Ok(bytes) => Ok(bytes),
                Err(actual) => {
                    Err(RuntimeError::expected::<$ty>(actual.type_info()))
                }
            }
        }

        $(#[$($meta)*])*
        ///
        /// This ensures that the value has read access to the underlying value
        /// and does not consume it.
        #[inline]
        pub fn $borrow_ref(&self) -> Result<BorrowRef<'_, $ty>, RuntimeError> {
            let result = BorrowRef::try_map(self.borrow_kind_ref()?, |kind| match kind {
                ValueKind::$kind(bytes) => Some(bytes),
                _ => None,
            });

            match result {
                Ok(bytes) => Ok(bytes),
                Err(actual) => {
                    Err(RuntimeError::expected::<$ty>(actual.type_info()))
                }
            }
        }

        $(#[$($meta)*])*
        ///
        /// This ensures that the value has write access to the underlying value
        /// and does not consume it.
        #[inline]
        pub fn $borrow_mut(&self) -> Result<BorrowMut<'_, $ty>, RuntimeError> {
            let result = BorrowMut::try_map(self.borrow_kind_mut()?, |kind| match kind {
                ValueKind::$kind(bytes) => Some(bytes),
                _ => None,
            });

            match result {
                Ok(bytes) => Ok(bytes),
                Err(actual) => {
                    Err(RuntimeError::expected::<$ty>(actual.type_info()))
                }
            }
        }
    }
}

macro_rules! into {
    (
        $(#[$($meta:meta)*])*
        $kind:ident($ty:ty),
        $into_ref:ident,
        $into_mut:ident,
        $borrow_ref:ident,
        $borrow_mut:ident,
        $into:ident,
    ) => {
        into_base! {
            $(#[$($meta)*])*
            $kind($ty),
            $into_ref,
            $into_mut,
            $borrow_ref,
            $borrow_mut,
        }

        $(#[$($meta)*])*
        ///
        /// This consumes the underlying value.
        #[inline]
        pub fn $into(self) -> Result<$ty, RuntimeError> {
            match self.take_kind()? {
                ValueKind::$kind(value) => Ok(value),
                actual => {
                    Err(RuntimeError::expected::<$ty>(actual.type_info()))
                }
            }
        }
    }
}

macro_rules! copy_into {
    (
        $(#[$($meta:meta)*])*
        $kind:ident($ty:ty),
        $into_ref:ident,
        $into_mut:ident,
        $borrow_ref:ident,
        $borrow_mut:ident,
        $as:ident,
    ) => {
        into_base! {
            $(#[$($meta)*])*
            $kind($ty),
            $into_ref,
            $into_mut,
            $borrow_ref,
            $borrow_mut,
        }

        $(#[$($meta)*])*
        ///
        /// This copied the underlying value.
        #[inline]
        pub fn $as(&self) -> Result<$ty, RuntimeError> {
            match *self.borrow_kind_ref()? {
                ValueKind::$kind(value) => Ok(value),
                ref actual => {
                    Err(RuntimeError::expected::<$ty>(actual.type_info()))
                }
            }
        }
    }
}

macro_rules! clone_into {
    (
        $(#[$($meta:meta)*])*
        $kind:ident($ty:ty),
        $into_ref:ident,
        $into_mut:ident,
        $borrow_ref:ident,
        $borrow_mut:ident,
        $as:ident,
    ) => {
        into_base! {
            $(#[$($meta)*])*
            $kind($ty),
            $into_ref,
            $into_mut,
            $borrow_ref,
            $borrow_mut,
        }

        $(#[$($meta)*])*
        ///
        /// This clones the underlying value.
        #[inline]
        pub fn $as(&self) -> Result<$ty, RuntimeError> {
            match &*self.borrow_kind_ref()? {
                ValueKind::$kind(value) => Ok(value.clone()),
                actual => {
                    Err(RuntimeError::expected::<$ty>(actual.type_info()))
                }
            }
        }
    }
}

// Small helper function to build errors.
fn err<T, E>(error: E) -> VmResult<T>
where
    VmErrorKind: From<E>,
{
    VmResult::err(error)
}

/// A empty with a well-defined type.
#[derive(TryClone)]
#[try_clone(crate)]
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
#[derive(TryClone)]
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
#[derive(TryClone)]
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
    pub fn get<Q>(&self, k: &Q) -> Option<&Value>
    where
        String: Borrow<Q>,
        Q: ?Sized + hash::Hash + Eq + Ord,
    {
        self.data.get(k)
    }

    /// Get the given mutable value by key in the object.
    pub fn get_mut<Q>(&mut self, k: &Q) -> Option<&mut Value>
    where
        String: Borrow<Q>,
        Q: ?Sized + hash::Hash + Eq + Ord,
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

#[derive(Clone)]
enum ValueRepr {
    Empty,
    Value(Shared<ValueKind>),
}

/// An entry on the stack.
#[derive(Clone)]
pub struct Value {
    repr: ValueRepr,
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
        let value = Shared::new(ValueKind::Any(AnyObj::from_ref(data)))?;
        let (value, guard) = Shared::into_drop_guard(value);
        Ok((
            Self {
                repr: ValueRepr::Value(value),
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
        let value = Shared::new(ValueKind::Any(obj))?;
        let (value, guard) = Shared::into_drop_guard(value);
        Ok((
            Self {
                repr: ValueRepr::Value(value),
            },
            guard,
        ))
    }

    /// Test if the value is writable.
    pub fn is_writable(&self) -> bool {
        match self.repr {
            ValueRepr::Empty => false,
            ValueRepr::Value(ref value) => value.is_writable(),
        }
    }

    /// Test if the value is readable.
    pub fn is_readable(&self) -> bool {
        match self.repr {
            ValueRepr::Empty => false,
            ValueRepr::Value(ref value) => value.is_readable(),
        }
    }

    /// Get snapshot of value.
    ///
    /// The snapshot details how the value is currently being access.
    pub fn snapshot(&self) -> Result<Snapshot, AccessError> {
        Ok(self.as_value_kind()?.snapshot())
    }

    /// Construct a unit value.
    pub(crate) fn unit() -> alloc::Result<Self> {
        Ok(Self {
            repr: ValueRepr::Value(Shared::new(ValueKind::EmptyTuple)?),
        })
    }

    /// Construct an empty value.
    pub(crate) const fn empty() -> Self {
        Self {
            repr: ValueRepr::Empty,
        }
    }

    /// Take the kind of the value.
    pub(crate) fn take_kind(self) -> Result<ValueKind, AccessError> {
        self.into_value_kind()?.take()
    }

    /// Borrow the kind of the value as a mutable reference.
    pub(crate) fn borrow_kind_mut(&self) -> Result<BorrowMut<'_, ValueKind>, AccessError> {
        self.as_value_kind()?.borrow_mut()
    }

    /// Take the kind of the value as an owned mutable reference.
    pub(crate) fn into_kind_mut(self) -> Result<Mut<ValueKind>, AccessError> {
        self.into_value_kind()?.into_mut()
    }

    /// Borrow the kind of the value as a reference.
    pub(crate) fn borrow_kind_ref(&self) -> Result<BorrowRef<'_, ValueKind>, AccessError> {
        self.as_value_kind()?.borrow_ref()
    }

    /// Take the kind of the value as an owned reference.
    pub(crate) fn into_kind_ref(self) -> Result<Ref<ValueKind>, AccessError> {
        self.into_value_kind()?.into_ref()
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
    pub(crate) fn string_display_with(
        &self,
        f: &mut Formatter,
        caller: &mut impl ProtocolCaller,
    ) -> VmResult<()> {
        match &*vm_try!(self.borrow_kind_ref()) {
            ValueKind::Char(c) => {
                vm_try!(f.push(*c));
            }
            ValueKind::Format(format) => {
                vm_try!(format.spec.format(&format.value, f, caller));
            }
            ValueKind::Integer(integer) => {
                let mut buffer = itoa::Buffer::new();
                vm_try!(f.push_str(buffer.format(*integer)));
            }
            ValueKind::Float(float) => {
                let mut buffer = ryu::Buffer::new();
                vm_try!(f.push_str(buffer.format(*float)));
            }
            ValueKind::Bool(bool) => {
                vm_write!(f, "{bool}");
            }
            ValueKind::Byte(byte) => {
                let mut buffer = itoa::Buffer::new();
                vm_try!(f.push_str(buffer.format(*byte)));
            }
            ValueKind::String(string) => {
                vm_try!(f.push_str(string));
            }
            _ => {
                let result =
                    vm_try!(caller.call_protocol_fn(Protocol::STRING_DISPLAY, self.clone(), (f,),));

                return VmResult::Ok(vm_try!(<()>::from_value(result)));
            }
        }

        VmResult::Ok(())
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

    pub(crate) fn clone_with(&self, caller: &mut impl ProtocolCaller) -> VmResult<Value> {
        let kind = match &*vm_try!(self.borrow_kind_ref()) {
            ValueKind::EmptyTuple => ValueKind::EmptyTuple,
            ValueKind::Bool(value) => ValueKind::Bool(*value),
            ValueKind::Byte(value) => ValueKind::Byte(*value),
            ValueKind::Char(value) => ValueKind::Char(*value),
            ValueKind::Integer(value) => ValueKind::Integer(*value),
            ValueKind::Float(value) => ValueKind::Float(*value),
            ValueKind::Type(value) => ValueKind::Type(*value),
            ValueKind::Ordering(value) => ValueKind::Ordering(*value),
            ValueKind::String(value) => ValueKind::String(vm_try!(value.try_clone())),
            ValueKind::Bytes(value) => ValueKind::Bytes(vm_try!(value.try_clone())),
            ValueKind::Vec(value) => ValueKind::Vec(vm_try!(value.try_clone())),
            ValueKind::Tuple(value) => ValueKind::Tuple(vm_try!(value.try_clone())),
            ValueKind::Object(value) => ValueKind::Object(vm_try!(value.try_clone())),
            ValueKind::RangeFrom(value) => ValueKind::RangeFrom(vm_try!(value.try_clone())),
            ValueKind::RangeFull(value) => ValueKind::RangeFull(vm_try!(value.try_clone())),
            ValueKind::RangeInclusive(value) => {
                ValueKind::RangeInclusive(vm_try!(value.try_clone()))
            }
            ValueKind::RangeToInclusive(value) => {
                ValueKind::RangeToInclusive(vm_try!(value.try_clone()))
            }
            ValueKind::RangeTo(value) => ValueKind::RangeTo(vm_try!(value.try_clone())),
            ValueKind::Range(value) => ValueKind::Range(vm_try!(value.try_clone())),
            ValueKind::ControlFlow(value) => ValueKind::ControlFlow(vm_try!(value.try_clone())),
            ValueKind::Stream(value) => ValueKind::Stream(vm_try!(value.try_clone())),
            ValueKind::Generator(value) => ValueKind::Generator(vm_try!(value.try_clone())),
            ValueKind::GeneratorState(value) => {
                ValueKind::GeneratorState(vm_try!(value.try_clone()))
            }
            ValueKind::Option(value) => ValueKind::Option(vm_try!(value.try_clone())),
            ValueKind::Result(value) => ValueKind::Result(vm_try!(value.try_clone())),
            ValueKind::EmptyStruct(value) => ValueKind::EmptyStruct(vm_try!(value.try_clone())),
            ValueKind::TupleStruct(value) => ValueKind::TupleStruct(vm_try!(value.try_clone())),
            ValueKind::Struct(value) => ValueKind::Struct(vm_try!(value.try_clone())),
            ValueKind::Variant(value) => ValueKind::Variant(vm_try!(value.try_clone())),
            ValueKind::Function(value) => ValueKind::Function(vm_try!(value.try_clone())),
            ValueKind::Format(value) => ValueKind::Format(vm_try!(value.try_clone())),
            _ => {
                return VmResult::Ok(vm_try!(caller.call_protocol_fn(
                    Protocol::CLONE,
                    self.clone(),
                    ()
                )));
            }
        };

        VmResult::Ok(Self {
            repr: ValueRepr::Value(vm_try!(Shared::new(kind))),
        })
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
        let value = match self.repr {
            ValueRepr::Empty => {
                vm_write!(f, "<empty>");
                return VmResult::Ok(());
            }
            ValueRepr::Value(ref value) => value,
        };

        match &*vm_try!(value.borrow_ref()) {
            ValueKind::EmptyTuple => {
                vm_write!(f, "()");
            }
            ValueKind::Bool(value) => {
                vm_write!(f, "{:?}", value);
            }
            ValueKind::Byte(value) => {
                vm_write!(f, "{:?}", value);
            }
            ValueKind::Char(value) => {
                vm_write!(f, "{:?}", value);
            }
            ValueKind::Integer(value) => {
                vm_write!(f, "{:?}", value);
            }
            ValueKind::Float(value) => {
                vm_write!(f, "{:?}", value);
            }
            ValueKind::Type(value) => {
                vm_write!(f, "{:?}", value);
            }
            ValueKind::String(value) => {
                vm_write!(f, "{:?}", value);
            }
            ValueKind::Bytes(value) => {
                vm_write!(f, "{:?}", value);
            }
            ValueKind::Vec(value) => {
                vm_try!(Vec::string_debug_with(value, f, caller));
            }
            ValueKind::Tuple(value) => {
                vm_write!(f, "{:?}", value);
            }
            ValueKind::Object(value) => {
                vm_write!(f, "{:?}", value);
            }
            ValueKind::RangeFrom(value) => {
                vm_write!(f, "{:?}", value);
            }
            ValueKind::RangeFull(value) => {
                vm_write!(f, "{:?}", value);
            }
            ValueKind::RangeInclusive(value) => {
                vm_write!(f, "{:?}", value);
            }
            ValueKind::RangeToInclusive(value) => {
                vm_write!(f, "{:?}", value);
            }
            ValueKind::RangeTo(value) => {
                vm_write!(f, "{:?}", value);
            }
            ValueKind::Range(value) => {
                vm_write!(f, "{:?}", value);
            }
            ValueKind::ControlFlow(value) => {
                vm_try!(ControlFlow::string_debug_with(value, f, caller));
            }
            ValueKind::Future(value) => {
                vm_write!(f, "{:?}", value);
            }
            ValueKind::Stream(value) => {
                vm_write!(f, "{:?}", value);
            }
            ValueKind::Generator(value) => {
                vm_write!(f, "{:?}", value);
            }
            ValueKind::GeneratorState(value) => {
                vm_write!(f, "{:?}", value);
            }
            ValueKind::Option(value) => {
                vm_write!(f, "{:?}", value);
            }
            ValueKind::Result(value) => {
                vm_write!(f, "{:?}", value);
            }
            ValueKind::EmptyStruct(value) => {
                vm_write!(f, "{:?}", value);
            }
            ValueKind::TupleStruct(value) => {
                vm_write!(f, "{:?}", value);
            }
            ValueKind::Struct(value) => {
                vm_write!(f, "{:?}", value);
            }
            ValueKind::Variant(value) => {
                vm_write!(f, "{:?}", value);
            }
            ValueKind::Function(value) => {
                vm_write!(f, "{:?}", value);
            }
            ValueKind::Format(value) => {
                vm_write!(f, "{:?}", value);
            }
            ValueKind::Iterator(value) => {
                vm_write!(f, "{:?}", value);
            }
            _ => {
                // reborrow f to avoid moving it
                let result =
                    caller.call_protocol_fn(Protocol::STRING_DEBUG, self.clone(), (&mut *f,));

                if let VmResult::Ok(result) = result {
                    vm_try!(<()>::from_value(result));
                } else {
                    let type_info = vm_try!(value.borrow_ref()).type_info();
                    vm_write!(f, "<{} object at {:p}>", type_info, value);
                }
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
        let value = vm_try!(caller.call_protocol_fn(Protocol::INTO_ITER, self, ()));
        Iterator::from_value(value)
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
        if let ValueRepr::Value(value) = self.repr {
            drop(vm_try!(value.take()));
        }

        VmResult::Ok(())
    }

    /// Move the interior value.
    pub(crate) fn move_(self) -> VmResult<Self> {
        match self.repr {
            ValueRepr::Empty => VmResult::Ok(Self::empty()),
            ValueRepr::Value(value) => VmResult::Ok(Value {
                repr: ValueRepr::Value(vm_try!(Shared::new(vm_try!(value.take())))),
            }),
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
    #[inline]
    pub fn borrow_string_ref(&self) -> Result<BorrowRef<'_, str>, RuntimeError> {
        let result = BorrowRef::try_map(self.borrow_kind_ref()?, |kind| match kind {
            ValueKind::String(string) => Some(string.as_str()),
            _ => None,
        });

        match result {
            Ok(s) => Ok(s),
            Err(actual) => Err(RuntimeError::expected::<String>(actual.type_info())),
        }
    }

    /// Take the current value as a string.
    #[inline]
    pub fn into_string(self) -> Result<String, RuntimeError> {
        match self.take_kind()? {
            ValueKind::String(string) => Ok(string),
            actual => Err(RuntimeError::expected::<String>(actual.type_info())),
        }
    }

    /// Coerce into type value.
    #[doc(hidden)]
    #[inline]
    pub fn into_type_value(self) -> Result<TypeValue, RuntimeError> {
        match self.take_kind()? {
            ValueKind::EmptyTuple => Ok(TypeValue::EmptyTuple),
            ValueKind::Tuple(tuple) => Ok(TypeValue::Tuple(tuple)),
            ValueKind::Object(object) => Ok(TypeValue::Object(object)),
            ValueKind::EmptyStruct(empty) => Ok(TypeValue::EmptyStruct(empty)),
            ValueKind::TupleStruct(tuple) => Ok(TypeValue::TupleStruct(tuple)),
            ValueKind::Struct(object) => Ok(TypeValue::Struct(object)),
            ValueKind::Variant(object) => Ok(TypeValue::Variant(object)),
            kind => Ok(TypeValue::NotTyped(NotTypedValueKind(kind))),
        }
    }

    /// Coerce into a unit.
    #[inline]
    pub fn into_unit(&self) -> Result<(), RuntimeError> {
        match *self.borrow_kind_ref()? {
            ValueKind::EmptyTuple => Ok(()),
            ref actual => Err(RuntimeError::expected::<()>(actual.type_info())),
        }
    }

    copy_into! {
        /// Coerce into [`Ordering`].
        Ordering(Ordering),
        into_ordering_ref,
        into_ordering_mut,
        borrow_ordering_ref,
        borrow_ordering_mut,
        as_ordering,
    }

    copy_into! {
        /// Coerce into [`bool`].
        Bool(bool),
        into_bool_ref,
        into_bool_mut,
        borrow_bool_ref,
        borrow_bool_mut,
        as_bool,
    }

    copy_into! {
        /// Coerce into [`u8`] byte.
        Byte(u8),
        into_byte_ref,
        into_byte_mut,
        borrow_byte_ref,
        borrow_byte_mut,
        as_byte,
    }

    copy_into! {
        /// Coerce into [`char`].
        Char(char),
        into_char_ref,
        into_char_mut,
        borrow_char_ref,
        borrow_char_mut,
        as_char,
    }

    copy_into! {
        /// Coerce into [`i64`] integer.
        Integer(i64),
        into_integer_ref,
        into_integer_mut,
        borrow_integer_ref,
        borrow_integer_mut,
        as_integer,
    }

    copy_into! {
        /// Coerce into [`f64`] float.
        Float(f64),
        into_float_ref,
        into_float_mut,
        borrow_float_ref,
        borrow_float_mut,
        as_float,
    }

    copy_into! {
        /// Coerce into [`Type`].
        Type(Type),
        into_type_ref,
        into_type_mut,
        borrow_type_ref,
        borrow_type_mut,
        as_type,
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
        /// Coerce into a [`Iterator`].
        Iterator(Iterator),
        into_iterator_ref,
        into_iterator_mut,
        borrow_iterator_ref,
        borrow_iterator_mut,
        into_iterator,
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
        match self.take_kind()? {
            ValueKind::Any(value) => Ok(value),
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
        let target = match vm_try!(self.take_kind()) {
            ValueKind::Future(future) => return VmResult::Ok(future),
            target => vm_try!(Value::try_from(target)),
        };

        let value = vm_try!(EnvProtocolCaller.call_protocol_fn(Protocol::INTO_FUTURE, target, ()));
        VmResult::Ok(vm_try!(Future::from_value(value)))
    }

    /// Try to coerce value into a typed reference.
    #[inline]
    pub fn into_any_ref<T>(self) -> Result<Ref<T>, RuntimeError>
    where
        T: Any,
    {
        let result = Ref::try_map(self.into_kind_ref()?, |kind| match kind {
            ValueKind::Any(any) => Some(any),
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
        let result = Mut::try_map(self.into_kind_mut()?, |kind| match kind {
            ValueKind::Any(any) => Some(any),
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
        let result = BorrowRef::try_map(self.borrow_kind_ref()?, |kind| match kind {
            ValueKind::Any(any) => any.downcast_borrow_ref().ok(),
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
        let result = BorrowMut::try_map(self.borrow_kind_mut()?, |kind| match kind {
            ValueKind::Any(any) => any.downcast_borrow_mut().ok(),
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
        let value = self.into_value_kind()?;

        let any = match value.take()? {
            ValueKind::Any(any) => any,
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
        Ok(self.borrow_kind_ref()?.type_hash())
    }

    /// Get the type information for the current value.
    pub fn type_info(&self) -> Result<TypeInfo, AccessError> {
        Ok(self.borrow_kind_ref()?.type_info())
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
    pub(crate) fn partial_eq_with(
        &self,
        b: &Value,
        caller: &mut impl ProtocolCaller,
    ) -> VmResult<bool> {
        {
            let a = vm_try!(self.borrow_kind_ref());

            match (&*a, &*vm_try!(b.borrow_kind_ref())) {
                (ValueKind::EmptyTuple, ValueKind::EmptyTuple) => return VmResult::Ok(true),
                (ValueKind::Bool(a), ValueKind::Bool(b)) => return VmResult::Ok(*a == *b),
                (ValueKind::Byte(a), ValueKind::Byte(b)) => return VmResult::Ok(*a == *b),
                (ValueKind::Char(a), ValueKind::Char(b)) => return VmResult::Ok(*a == *b),
                (ValueKind::Integer(a), ValueKind::Integer(b)) => return VmResult::Ok(*a == *b),
                (ValueKind::Float(a), ValueKind::Float(b)) => return VmResult::Ok(*a == *b),
                (ValueKind::Type(a), ValueKind::Type(b)) => return VmResult::Ok(*a == *b),
                (ValueKind::Bytes(a), ValueKind::Bytes(b)) => {
                    return VmResult::Ok(*a == *b);
                }
                (ValueKind::RangeFrom(a), ValueKind::RangeFrom(b)) => {
                    return RangeFrom::partial_eq_with(a, b, caller);
                }
                (ValueKind::RangeFull(a), ValueKind::RangeFull(b)) => {
                    return RangeFull::partial_eq_with(a, b, caller);
                }
                (ValueKind::RangeInclusive(a), ValueKind::RangeInclusive(b)) => {
                    return RangeInclusive::partial_eq_with(a, b, caller);
                }
                (ValueKind::RangeToInclusive(a), ValueKind::RangeToInclusive(b)) => {
                    return RangeToInclusive::partial_eq_with(a, b, caller);
                }
                (ValueKind::RangeTo(a), ValueKind::RangeTo(b)) => {
                    return RangeTo::partial_eq_with(a, b, caller);
                }
                (ValueKind::Range(a), ValueKind::Range(b)) => {
                    return Range::partial_eq_with(a, b, caller);
                }
                (ValueKind::ControlFlow(a), ValueKind::ControlFlow(b)) => {
                    return ControlFlow::partial_eq_with(a, b, caller);
                }
                (ValueKind::EmptyStruct(a), ValueKind::EmptyStruct(b)) => {
                    if a.rtti.hash == b.rtti.hash {
                        // NB: don't get any future ideas, this must fall through to
                        // the VmError below since it's otherwise a comparison
                        // between two incompatible types.
                        //
                        // Other than that, all units are equal.
                        return VmResult::Ok(true);
                    }
                }
                (ValueKind::TupleStruct(a), ValueKind::TupleStruct(b)) => {
                    if a.rtti.hash == b.rtti.hash {
                        return Vec::eq_with(&a.data, &b.data, Value::partial_eq_with, caller);
                    }
                }
                (ValueKind::Struct(a), ValueKind::Struct(b)) => {
                    if a.rtti.hash == b.rtti.hash {
                        return Object::eq_with(&a.data, &b.data, Value::partial_eq_with, caller);
                    }
                }
                (ValueKind::Variant(a), ValueKind::Variant(b)) => {
                    if a.rtti().enum_hash == b.rtti().enum_hash {
                        return Variant::partial_eq_with(a, b, caller);
                    }
                }
                (ValueKind::String(a), ValueKind::String(b)) => {
                    return VmResult::Ok(*a == *b);
                }
                (ValueKind::Option(a), ValueKind::Option(b)) => match (a, b) {
                    (Some(a), Some(b)) => return Value::partial_eq_with(a, b, caller),
                    (None, None) => return VmResult::Ok(true),
                    _ => return VmResult::Ok(false),
                },
                (ValueKind::Result(a), ValueKind::Result(b)) => match (a, b) {
                    (Ok(a), Ok(b)) => return Value::partial_eq_with(a, b, caller),
                    (Err(a), Err(b)) => return Value::partial_eq_with(a, b, caller),
                    _ => return VmResult::Ok(false),
                },
                _ => {}
            }

            match &*a {
                ValueKind::Vec(a) => {
                    return Vec::partial_eq_with(a, b.clone(), caller);
                }
                ValueKind::Tuple(a) => {
                    return Vec::partial_eq_with(a, b.clone(), caller);
                }
                ValueKind::Object(a) => {
                    return Object::partial_eq_with(a, b.clone(), caller);
                }
                _ => {}
            }
        }

        if let CallResultOnly::Ok(value) =
            vm_try!(caller.try_call_protocol_fn(Protocol::PARTIAL_EQ, self.clone(), (b.clone(),)))
        {
            return <_>::from_value(value);
        }

        err(VmErrorKind::UnsupportedBinaryOperation {
            op: "partial_eq",
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
    pub(crate) fn hash_with(
        &self,
        hasher: &mut Hasher,
        caller: &mut impl ProtocolCaller,
    ) -> VmResult<()> {
        match &*vm_try!(self.borrow_kind_ref()) {
            ValueKind::Integer(value) => {
                hasher.write_i64(*value);
                return VmResult::Ok(());
            }
            ValueKind::Byte(value) => {
                hasher.write_u8(*value);
                return VmResult::Ok(());
            }
            // Care must be taken whan hashing floats, to ensure that `hash(v1)
            // === hash(v2)` if `eq(v1) === eq(v2)`. Hopefully we accomplish
            // this by rejecting NaNs and rectifying subnormal values of zero.
            ValueKind::Float(value) => {
                if value.is_nan() {
                    return VmResult::err(VmErrorKind::IllegalFloatOperation { value: *value });
                }

                let zero = *value == 0.0;
                hasher.write_f64((zero as u8 as f64) * 0.0 + (!zero as u8 as f64) * *value);
                return VmResult::Ok(());
            }
            ValueKind::String(string) => {
                hasher.write_str(string);
                return VmResult::Ok(());
            }
            ValueKind::Bytes(bytes) => {
                hasher.write(bytes);
                return VmResult::Ok(());
            }
            ValueKind::Tuple(tuple) => {
                return Tuple::hash_with(tuple, hasher, caller);
            }
            ValueKind::Vec(vec) => {
                return Vec::hash_with(vec, hasher, caller);
            }
            _ => {}
        }

        if let CallResultOnly::Ok(value) =
            vm_try!(caller.try_call_protocol_fn(Protocol::HASH, self.clone(), (hasher,)))
        {
            return <_>::from_value(value);
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
        match (
            &*vm_try!(self.borrow_kind_ref()),
            &*vm_try!(b.borrow_kind_ref()),
        ) {
            (ValueKind::Bool(a), ValueKind::Bool(b)) => return VmResult::Ok(*a == *b),
            (ValueKind::Byte(a), ValueKind::Byte(b)) => return VmResult::Ok(*a == *b),
            (ValueKind::Char(a), ValueKind::Char(b)) => return VmResult::Ok(*a == *b),
            (ValueKind::Float(a), ValueKind::Float(b)) => {
                let Some(ordering) = a.partial_cmp(b) else {
                    return VmResult::err(VmErrorKind::IllegalFloatComparison { lhs: *a, rhs: *b });
                };

                return VmResult::Ok(matches!(ordering, Ordering::Equal));
            }
            (ValueKind::Integer(a), ValueKind::Integer(b)) => return VmResult::Ok(*a == *b),
            (ValueKind::Type(a), ValueKind::Type(b)) => return VmResult::Ok(*a == *b),
            (ValueKind::Bytes(a), ValueKind::Bytes(b)) => {
                return VmResult::Ok(*a == *b);
            }
            (ValueKind::Vec(a), ValueKind::Vec(b)) => {
                return Vec::eq_with(a, b, Value::eq_with, caller);
            }
            (ValueKind::EmptyTuple, ValueKind::EmptyTuple) => return VmResult::Ok(true),
            (ValueKind::Tuple(a), ValueKind::Tuple(b)) => {
                return Vec::eq_with(a, b, Value::eq_with, caller);
            }
            (ValueKind::Object(a), ValueKind::Object(b)) => {
                return Object::eq_with(a, b, Value::eq_with, caller);
            }
            (ValueKind::RangeFrom(a), ValueKind::RangeFrom(b)) => {
                return RangeFrom::eq_with(a, b, caller);
            }
            (ValueKind::RangeFull(a), ValueKind::RangeFull(b)) => {
                return RangeFull::eq_with(a, b, caller);
            }
            (ValueKind::RangeInclusive(a), ValueKind::RangeInclusive(b)) => {
                return RangeInclusive::eq_with(a, b, caller);
            }
            (ValueKind::RangeToInclusive(a), ValueKind::RangeToInclusive(b)) => {
                return RangeToInclusive::eq_with(a, b, caller);
            }
            (ValueKind::RangeTo(a), ValueKind::RangeTo(b)) => {
                return RangeTo::eq_with(a, b, caller);
            }
            (ValueKind::Range(a), ValueKind::Range(b)) => {
                return Range::eq_with(a, b, caller);
            }
            (ValueKind::ControlFlow(a), ValueKind::ControlFlow(b)) => {
                return ControlFlow::eq_with(a, b, caller);
            }
            (ValueKind::EmptyStruct(a), ValueKind::EmptyStruct(b)) => {
                if a.rtti.hash == b.rtti.hash {
                    // NB: don't get any future ideas, this must fall through to
                    // the VmError below since it's otherwise a comparison
                    // between two incompatible types.
                    //
                    // Other than that, all units are equal.
                    return VmResult::Ok(true);
                }
            }
            (ValueKind::TupleStruct(a), ValueKind::TupleStruct(b)) => {
                if a.rtti.hash == b.rtti.hash {
                    return Vec::eq_with(&a.data, &b.data, Value::eq_with, caller);
                }
            }
            (ValueKind::Struct(a), ValueKind::Struct(b)) => {
                if a.rtti.hash == b.rtti.hash {
                    return Object::eq_with(&a.data, &b.data, Value::eq_with, caller);
                }
            }
            (ValueKind::Variant(a), ValueKind::Variant(b)) => {
                if a.rtti().enum_hash == b.rtti().enum_hash {
                    return Variant::eq_with(a, b, caller);
                }
            }
            (ValueKind::String(a), ValueKind::String(b)) => {
                return VmResult::Ok(*a == *b);
            }
            (ValueKind::Option(a), ValueKind::Option(b)) => match (a, b) {
                (Some(a), Some(b)) => return Value::eq_with(a, b, caller),
                (None, None) => return VmResult::Ok(true),
                _ => return VmResult::Ok(false),
            },
            (ValueKind::Result(a), ValueKind::Result(b)) => match (a, b) {
                (Ok(a), Ok(b)) => return Value::eq_with(a, b, caller),
                (Err(a), Err(b)) => return Value::eq_with(a, b, caller),
                _ => return VmResult::Ok(false),
            },
            _ => {}
        }

        if let CallResultOnly::Ok(value) =
            vm_try!(caller.try_call_protocol_fn(Protocol::EQ, self.clone(), (b.clone(),)))
        {
            return <_>::from_value(value);
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
        &self,
        b: &Value,
        caller: &mut impl ProtocolCaller,
    ) -> VmResult<Option<Ordering>> {
        match (
            &*vm_try!(self.borrow_kind_ref()),
            &*vm_try!(b.borrow_kind_ref()),
        ) {
            (ValueKind::EmptyTuple, ValueKind::EmptyTuple) => {
                return VmResult::Ok(Some(Ordering::Equal))
            }
            (ValueKind::Bool(a), ValueKind::Bool(b)) => return VmResult::Ok(a.partial_cmp(b)),
            (ValueKind::Byte(a), ValueKind::Byte(b)) => return VmResult::Ok(a.partial_cmp(b)),
            (ValueKind::Char(a), ValueKind::Char(b)) => return VmResult::Ok(a.partial_cmp(b)),
            (ValueKind::Float(a), ValueKind::Float(b)) => return VmResult::Ok(a.partial_cmp(b)),
            (ValueKind::Integer(a), ValueKind::Integer(b)) => {
                return VmResult::Ok(a.partial_cmp(b));
            }
            (ValueKind::Type(a), ValueKind::Type(b)) => return VmResult::Ok(a.partial_cmp(b)),
            (ValueKind::Bytes(a), ValueKind::Bytes(b)) => {
                return VmResult::Ok(a.partial_cmp(b));
            }
            (ValueKind::Vec(a), ValueKind::Vec(b)) => {
                return Vec::partial_cmp_with(a, b, caller);
            }
            (ValueKind::Tuple(a), ValueKind::Tuple(b)) => {
                return Vec::partial_cmp_with(a, b, caller);
            }
            (ValueKind::Object(a), ValueKind::Object(b)) => {
                return Object::partial_cmp_with(a, b, caller);
            }
            (ValueKind::RangeFrom(a), ValueKind::RangeFrom(b)) => {
                return RangeFrom::partial_cmp_with(a, b, caller);
            }
            (ValueKind::RangeFull(a), ValueKind::RangeFull(b)) => {
                return RangeFull::partial_cmp_with(a, b, caller);
            }
            (ValueKind::RangeInclusive(a), ValueKind::RangeInclusive(b)) => {
                return RangeInclusive::partial_cmp_with(a, b, caller);
            }
            (ValueKind::RangeToInclusive(a), ValueKind::RangeToInclusive(b)) => {
                return RangeToInclusive::partial_cmp_with(a, b, caller);
            }
            (ValueKind::RangeTo(a), ValueKind::RangeTo(b)) => {
                return RangeTo::partial_cmp_with(a, b, caller);
            }
            (ValueKind::Range(a), ValueKind::Range(b)) => {
                return Range::partial_cmp_with(a, b, caller);
            }
            (ValueKind::EmptyStruct(a), ValueKind::EmptyStruct(b)) => {
                if a.rtti.hash == b.rtti.hash {
                    // NB: don't get any future ideas, this must fall through to
                    // the VmError below since it's otherwise a comparison
                    // between two incompatible types.
                    //
                    // Other than that, all units are equal.
                    return VmResult::Ok(Some(Ordering::Equal));
                }
            }
            (ValueKind::TupleStruct(a), ValueKind::TupleStruct(b)) => {
                if a.rtti.hash == b.rtti.hash {
                    return Vec::partial_cmp_with(&a.data, &b.data, caller);
                }
            }
            (ValueKind::Struct(a), ValueKind::Struct(b)) => {
                if a.rtti.hash == b.rtti.hash {
                    return Object::partial_cmp_with(&a.data, &b.data, caller);
                }
            }
            (ValueKind::Variant(a), ValueKind::Variant(b)) => {
                if a.rtti().enum_hash == b.rtti().enum_hash {
                    return Variant::partial_cmp_with(a, b, caller);
                }
            }
            (ValueKind::String(a), ValueKind::String(b)) => {
                return VmResult::Ok(a.partial_cmp(b));
            }
            (ValueKind::Option(a), ValueKind::Option(b)) => match (a, b) {
                (Some(a), Some(b)) => return Value::partial_cmp_with(a, b, caller),
                (None, None) => return VmResult::Ok(Some(Ordering::Equal)),
                (Some(..), None) => return VmResult::Ok(Some(Ordering::Greater)),
                (None, Some(..)) => return VmResult::Ok(Some(Ordering::Less)),
            },
            (ValueKind::Result(a), ValueKind::Result(b)) => match (a, b) {
                (Ok(a), Ok(b)) => return Value::partial_cmp_with(a, b, caller),
                (Err(a), Err(b)) => return Value::partial_cmp_with(a, b, caller),
                (Ok(..), Err(..)) => return VmResult::Ok(Some(Ordering::Greater)),
                (Err(..), Ok(..)) => return VmResult::Ok(Some(Ordering::Less)),
            },
            _ => {}
        }

        if let CallResultOnly::Ok(value) =
            vm_try!(caller.try_call_protocol_fn(Protocol::PARTIAL_CMP, self.clone(), (b.clone(),)))
        {
            return <_>::from_value(value);
        }

        err(VmErrorKind::UnsupportedBinaryOperation {
            op: "partial_cmp",
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
    pub(crate) fn cmp_with(
        &self,
        b: &Value,
        caller: &mut impl ProtocolCaller,
    ) -> VmResult<Ordering> {
        match (
            &*vm_try!(self.borrow_kind_ref()),
            &*vm_try!(b.borrow_kind_ref()),
        ) {
            (ValueKind::EmptyTuple, ValueKind::EmptyTuple) => return VmResult::Ok(Ordering::Equal),
            (ValueKind::Bool(a), ValueKind::Bool(b)) => return VmResult::Ok(a.cmp(b)),
            (ValueKind::Byte(a), ValueKind::Byte(b)) => return VmResult::Ok(a.cmp(b)),
            (ValueKind::Char(a), ValueKind::Char(b)) => return VmResult::Ok(a.cmp(b)),
            (ValueKind::Float(a), ValueKind::Float(b)) => {
                let Some(ordering) = a.partial_cmp(b) else {
                    return VmResult::err(VmErrorKind::IllegalFloatComparison { lhs: *a, rhs: *b });
                };

                return VmResult::Ok(ordering);
            }
            (ValueKind::Integer(a), ValueKind::Integer(b)) => return VmResult::Ok(a.cmp(b)),
            (ValueKind::Type(a), ValueKind::Type(b)) => return VmResult::Ok(a.cmp(b)),
            (ValueKind::Bytes(a), ValueKind::Bytes(b)) => {
                return VmResult::Ok(a.cmp(b));
            }
            (ValueKind::Vec(a), ValueKind::Vec(b)) => {
                return Vec::cmp_with(a, b, caller);
            }
            (ValueKind::Tuple(a), ValueKind::Tuple(b)) => {
                return Vec::cmp_with(a, b, caller);
            }
            (ValueKind::Object(a), ValueKind::Object(b)) => {
                return Object::cmp_with(a, b, caller);
            }
            (ValueKind::RangeFrom(a), ValueKind::RangeFrom(b)) => {
                return RangeFrom::cmp_with(a, b, caller);
            }
            (ValueKind::RangeFull(a), ValueKind::RangeFull(b)) => {
                return RangeFull::cmp_with(a, b, caller);
            }
            (ValueKind::RangeInclusive(a), ValueKind::RangeInclusive(b)) => {
                return RangeInclusive::cmp_with(a, b, caller);
            }
            (ValueKind::RangeToInclusive(a), ValueKind::RangeToInclusive(b)) => {
                return RangeToInclusive::cmp_with(a, b, caller);
            }
            (ValueKind::RangeTo(a), ValueKind::RangeTo(b)) => {
                return RangeTo::cmp_with(a, b, caller);
            }
            (ValueKind::Range(a), ValueKind::Range(b)) => {
                return Range::cmp_with(a, b, caller);
            }
            (ValueKind::EmptyStruct(a), ValueKind::EmptyStruct(b)) => {
                if a.rtti.hash == b.rtti.hash {
                    // NB: don't get any future ideas, this must fall through to
                    // the VmError below since it's otherwise a comparison
                    // between two incompatible types.
                    //
                    // Other than that, all units are equal.
                    return VmResult::Ok(Ordering::Equal);
                }
            }
            (ValueKind::TupleStruct(a), ValueKind::TupleStruct(b)) => {
                if a.rtti.hash == b.rtti.hash {
                    return Vec::cmp_with(&a.data, &b.data, caller);
                }
            }
            (ValueKind::Struct(a), ValueKind::Struct(b)) => {
                if a.rtti.hash == b.rtti.hash {
                    return Object::cmp_with(&a.data, &b.data, caller);
                }
            }
            (ValueKind::Variant(a), ValueKind::Variant(b)) => {
                if a.rtti().enum_hash == b.rtti().enum_hash {
                    return Variant::cmp_with(a, b, caller);
                }
            }
            (ValueKind::String(a), ValueKind::String(b)) => {
                return VmResult::Ok(a.cmp(b));
            }
            (ValueKind::Option(a), ValueKind::Option(b)) => match (a, b) {
                (Some(a), Some(b)) => return Value::cmp_with(a, b, caller),
                (None, None) => return VmResult::Ok(Ordering::Equal),
                (Some(..), None) => return VmResult::Ok(Ordering::Greater),
                (None, Some(..)) => return VmResult::Ok(Ordering::Less),
            },
            (ValueKind::Result(a), ValueKind::Result(b)) => match (a, b) {
                (Ok(a), Ok(b)) => return Value::cmp_with(a, b, caller),
                (Err(a), Err(b)) => return Value::cmp_with(a, b, caller),
                (Ok(..), Err(..)) => return VmResult::Ok(Ordering::Greater),
                (Err(..), Ok(..)) => return VmResult::Ok(Ordering::Less),
            },
            _ => {}
        }

        if let CallResultOnly::Ok(value) =
            vm_try!(caller.try_call_protocol_fn(Protocol::CMP, self.clone(), (b.clone(),)))
        {
            return <_>::from_value(value);
        }

        err(VmErrorKind::UnsupportedBinaryOperation {
            op: "cmp",
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

    fn into_value_kind(self) -> Result<Shared<ValueKind>, AccessError> {
        match self.repr {
            ValueRepr::Value(value) => Ok(value),
            ValueRepr::Empty => Err(AccessError::empty()),
        }
    }

    fn as_value_kind(&self) -> Result<&Shared<ValueKind>, AccessError> {
        match &self.repr {
            ValueRepr::Value(value) => Ok(value),
            ValueRepr::Empty => Err(AccessError::empty()),
        }
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let snapshot = match &self.repr {
            ValueRepr::Empty => {
                write!(f, "<empty>")?;
                return Ok(());
            }
            ValueRepr::Value(value) => value.snapshot(),
        };

        if !snapshot.is_readable() {
            write!(f, "<{snapshot}>")?;
            return Ok(());
        }

        let mut o = Formatter::new();

        if self.string_debug(&mut o).is_err() {
            match self.type_info() {
                Ok(type_info) => {
                    write!(f, "<{} object at {:p}>", type_info, self)?;
                }
                Err(e) => {
                    write!(f, "<unknown object at {:p}: {}>", self, e)?;
                }
            }
            return Ok(());
        }

        f.write_str(o.as_str())?;
        Ok(())
    }
}

impl TryFrom<()> for Value {
    type Error = alloc::Error;

    #[inline]
    fn try_from((): ()) -> Result<Self, Self::Error> {
        Value::try_from(ValueKind::EmptyTuple)
    }
}

impl IntoOutput for () {
    type Output = ();

    #[inline]
    fn into_output(self) -> VmResult<Self::Output> {
        VmResult::Ok(())
    }
}

impl TryFrom<ValueKind> for Value {
    type Error = alloc::Error;

    #[inline]
    fn try_from(kind: ValueKind) -> Result<Self, Self::Error> {
        Ok(Self {
            repr: ValueRepr::Value(Shared::new(kind)?),
        })
    }
}

impl ToValue for Value {
    #[inline]
    fn to_value(self) -> VmResult<Value> {
        VmResult::Ok(self)
    }
}

macro_rules! impl_from {
    ($($variant:ident => $ty:ty),* $(,)*) => {
        $(
            impl TryFrom<$ty> for Value {
                type Error = alloc::Error;

                #[inline]
                fn try_from(value: $ty) -> Result<Self, Self::Error> {
                    Value::try_from(ValueKind::$variant(value))
                }
            }

            impl IntoOutput for $ty {
                type Output = $ty;

                #[inline]
                fn into_output(self) -> VmResult<Self::Output> {
                    VmResult::Ok(self)
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

macro_rules! impl_custom_from_wrapper {
    ($($variant:ident => $ty:ty),* $(,)?) => {
        $(
            impl TryFrom<$ty> for Value {
                type Error = alloc::Error;

                #[inline]
                fn try_from(value: $ty) -> Result<Self, alloc::Error> {
                    Value::try_from(ValueKind::$variant(value))
                }
            }

            impl IntoOutput for $ty {
                type Output = $ty;

                #[inline]
                fn into_output(self) -> VmResult<Self::Output> {
                    VmResult::Ok(self)
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
    Ordering => Ordering,
    String => String,
    Bytes => Bytes,
    ControlFlow => ControlFlow,
    Function => Function,
    Iterator => Iterator,
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

impl_custom_from_wrapper! {
    Option => Option<Value>,
    Result => Result<Value, Value>,
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

/// Wrapper for a value kind.
#[doc(hidden)]
pub struct NotTypedValueKind(ValueKind);

/// The coersion of a value into a typed value.
#[doc(hidden)]
#[non_exhaustive]
pub enum TypeValue {
    /// The unit value.
    EmptyTuple,
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
    /// Not a typed value.
    #[doc(hidden)]
    NotTyped(NotTypedValueKind),
}

impl TypeValue {
    /// Get the type info of the current value.
    #[doc(hidden)]
    pub fn type_info(&self) -> TypeInfo {
        match self {
            TypeValue::EmptyTuple => TypeInfo::StaticType(crate::runtime::static_type::TUPLE_TYPE),
            TypeValue::Tuple(..) => TypeInfo::StaticType(crate::runtime::static_type::TUPLE_TYPE),
            TypeValue::Object(..) => TypeInfo::StaticType(crate::runtime::static_type::OBJECT_TYPE),
            TypeValue::EmptyStruct(empty) => empty.type_info(),
            TypeValue::TupleStruct(tuple) => tuple.type_info(),
            TypeValue::Struct(object) => object.type_info(),
            TypeValue::Variant(empty) => empty.type_info(),
            TypeValue::NotTyped(kind) => kind.0.type_info(),
        }
    }
}

#[doc(hidden)]
#[non_exhaustive]
pub(crate) enum ValueKind {
    /// The unit value.
    EmptyTuple,
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
    /// An iterator.
    Iterator(Iterator),
    /// An opaque value that can be downcasted.
    Any(AnyObj),
}

impl ValueKind {
    pub(crate) fn type_info(&self) -> TypeInfo {
        match self {
            ValueKind::Bool(..) => TypeInfo::StaticType(crate::runtime::static_type::BOOL_TYPE),
            ValueKind::Byte(..) => TypeInfo::StaticType(crate::runtime::static_type::BYTE_TYPE),
            ValueKind::Char(..) => TypeInfo::StaticType(crate::runtime::static_type::CHAR_TYPE),
            ValueKind::Integer(..) => {
                TypeInfo::StaticType(crate::runtime::static_type::INTEGER_TYPE)
            }
            ValueKind::Float(..) => TypeInfo::StaticType(crate::runtime::static_type::FLOAT_TYPE),
            ValueKind::Type(..) => TypeInfo::StaticType(crate::runtime::static_type::TYPE),
            ValueKind::Ordering(..) => {
                TypeInfo::StaticType(crate::runtime::static_type::ORDERING_TYPE)
            }
            ValueKind::String(..) => TypeInfo::StaticType(crate::runtime::static_type::STRING_TYPE),
            ValueKind::Bytes(..) => TypeInfo::StaticType(crate::runtime::static_type::BYTES_TYPE),
            ValueKind::Vec(..) => TypeInfo::StaticType(crate::runtime::static_type::VEC_TYPE),
            ValueKind::EmptyTuple => TypeInfo::StaticType(crate::runtime::static_type::TUPLE_TYPE),
            ValueKind::Tuple(..) => TypeInfo::StaticType(crate::runtime::static_type::TUPLE_TYPE),
            ValueKind::Object(..) => TypeInfo::StaticType(crate::runtime::static_type::OBJECT_TYPE),
            ValueKind::RangeFrom(..) => {
                TypeInfo::StaticType(crate::runtime::static_type::RANGE_FROM_TYPE)
            }
            ValueKind::RangeFull(..) => {
                TypeInfo::StaticType(crate::runtime::static_type::RANGE_FULL_TYPE)
            }
            ValueKind::RangeInclusive(..) => {
                TypeInfo::StaticType(crate::runtime::static_type::RANGE_INCLUSIVE_TYPE)
            }
            ValueKind::RangeToInclusive(..) => {
                TypeInfo::StaticType(crate::runtime::static_type::RANGE_TO_INCLUSIVE_TYPE)
            }
            ValueKind::RangeTo(..) => {
                TypeInfo::StaticType(crate::runtime::static_type::RANGE_TO_TYPE)
            }
            ValueKind::Range(..) => TypeInfo::StaticType(crate::runtime::static_type::RANGE_TYPE),
            ValueKind::ControlFlow(..) => {
                TypeInfo::StaticType(crate::runtime::static_type::CONTROL_FLOW_TYPE)
            }
            ValueKind::Future(..) => TypeInfo::StaticType(crate::runtime::static_type::FUTURE_TYPE),
            ValueKind::Stream(..) => TypeInfo::StaticType(crate::runtime::static_type::STREAM_TYPE),
            ValueKind::Generator(..) => {
                TypeInfo::StaticType(crate::runtime::static_type::GENERATOR_TYPE)
            }
            ValueKind::GeneratorState(..) => {
                TypeInfo::StaticType(crate::runtime::static_type::GENERATOR_STATE_TYPE)
            }
            ValueKind::Option(..) => TypeInfo::StaticType(crate::runtime::static_type::OPTION_TYPE),
            ValueKind::Result(..) => TypeInfo::StaticType(crate::runtime::static_type::RESULT_TYPE),
            ValueKind::Function(..) => {
                TypeInfo::StaticType(crate::runtime::static_type::FUNCTION_TYPE)
            }
            ValueKind::Format(..) => TypeInfo::StaticType(crate::runtime::static_type::FORMAT_TYPE),
            ValueKind::Iterator(..) => {
                TypeInfo::StaticType(crate::runtime::static_type::ITERATOR_TYPE)
            }
            ValueKind::EmptyStruct(empty) => empty.type_info(),
            ValueKind::TupleStruct(tuple) => tuple.type_info(),
            ValueKind::Struct(object) => object.type_info(),
            ValueKind::Variant(empty) => empty.type_info(),
            ValueKind::Any(any) => any.type_info(),
        }
    }

    /// Get the type hash for the current value.
    ///
    /// One notable feature is that the type of a variant is its container
    /// *enum*, and not the type hash of the variant itself.
    pub(crate) fn type_hash(&self) -> Hash {
        match self {
            ValueKind::Bool(..) => crate::runtime::static_type::BOOL_TYPE.hash,
            ValueKind::Byte(..) => crate::runtime::static_type::BYTE_TYPE.hash,
            ValueKind::Char(..) => crate::runtime::static_type::CHAR_TYPE.hash,
            ValueKind::Integer(..) => crate::runtime::static_type::INTEGER_TYPE.hash,
            ValueKind::Float(..) => crate::runtime::static_type::FLOAT_TYPE.hash,
            ValueKind::Type(..) => crate::runtime::static_type::TYPE.hash,
            ValueKind::Ordering(..) => crate::runtime::static_type::ORDERING_TYPE.hash,
            ValueKind::String(..) => crate::runtime::static_type::STRING_TYPE.hash,
            ValueKind::Bytes(..) => crate::runtime::static_type::BYTES_TYPE.hash,
            ValueKind::Vec(..) => crate::runtime::static_type::VEC_TYPE.hash,
            ValueKind::EmptyTuple => crate::runtime::static_type::TUPLE_TYPE.hash,
            ValueKind::Tuple(..) => crate::runtime::static_type::TUPLE_TYPE.hash,
            ValueKind::Object(..) => crate::runtime::static_type::OBJECT_TYPE.hash,
            ValueKind::RangeFrom(..) => crate::runtime::static_type::RANGE_FROM_TYPE.hash,
            ValueKind::RangeFull(..) => crate::runtime::static_type::RANGE_FULL_TYPE.hash,
            ValueKind::RangeInclusive(..) => crate::runtime::static_type::RANGE_INCLUSIVE_TYPE.hash,
            ValueKind::RangeToInclusive(..) => {
                crate::runtime::static_type::RANGE_TO_INCLUSIVE_TYPE.hash
            }
            ValueKind::RangeTo(..) => crate::runtime::static_type::RANGE_TO_TYPE.hash,
            ValueKind::Range(..) => crate::runtime::static_type::RANGE_TYPE.hash,
            ValueKind::ControlFlow(..) => crate::runtime::static_type::CONTROL_FLOW_TYPE.hash,
            ValueKind::Future(..) => crate::runtime::static_type::FUTURE_TYPE.hash,
            ValueKind::Stream(..) => crate::runtime::static_type::STREAM_TYPE.hash,
            ValueKind::Generator(..) => crate::runtime::static_type::GENERATOR_TYPE.hash,
            ValueKind::GeneratorState(..) => crate::runtime::static_type::GENERATOR_STATE_TYPE.hash,
            ValueKind::Result(..) => crate::runtime::static_type::RESULT_TYPE.hash,
            ValueKind::Option(..) => crate::runtime::static_type::OPTION_TYPE.hash,
            ValueKind::Function(..) => crate::runtime::static_type::FUNCTION_TYPE.hash,
            ValueKind::Format(..) => crate::runtime::static_type::FORMAT_TYPE.hash,
            ValueKind::Iterator(..) => crate::runtime::static_type::ITERATOR_TYPE.hash,
            ValueKind::EmptyStruct(empty) => empty.rtti.hash,
            ValueKind::TupleStruct(tuple) => tuple.rtti.hash,
            ValueKind::Struct(object) => object.rtti.hash,
            ValueKind::Variant(variant) => variant.rtti().enum_hash,
            ValueKind::Any(any) => any.type_hash(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Value;

    #[test]
    fn test_size() {
        assert_eq! {
            std::mem::size_of::<Value>(),
            std::mem::size_of::<usize>(),
        };
    }
}
