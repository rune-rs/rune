#[macro_use]
mod macros;

use core::any;
use core::cmp::Ordering;
use core::fmt;

use rust_alloc::sync::Arc;

#[cfg(feature = "musli")]
use musli::{Decode, Encode};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate as rune;
use crate::alloc::prelude::*;
use crate::alloc::{self, HashMap};
use crate::runtime;
use crate::{Hash, TypeHash};

use super::{
    Bytes, FromValue, Inline, Object, OwnedTuple, Repr, ToValue, Tuple, Type, TypeInfo, Value,
    VmErrorKind,
};

/// Derive for the [`ToConstValue`] trait.
///
/// This is principally used for associated constants in native modules, since
/// Rune has to be provided a constant-compatible method for constructing values
/// of the given type.
///
/// [`ToConstValue`]: trait@crate::ToConstValue
///
/// # Examples
///
/// ```
/// use rune::{docstring, Any, ContextError, Module, ToConstValue};
///
/// #[derive(Any, ToConstValue)]
/// pub struct Duration {
///     #[const_value(with = const_duration)]
///     inner: std::time::Duration,
/// }
///
/// mod const_duration {
///     use rune::runtime::{ConstValue, RuntimeError, Value};
///     use std::time::Duration;
///
///     #[inline]
///     pub(super) fn to_const_value(duration: Duration) -> Result<ConstValue, RuntimeError> {
///         let secs = duration.as_secs();
///         let nanos = duration.subsec_nanos();
///         rune::to_const_value((secs, nanos))
///     }
///
///     #[inline]
///     pub(super) fn from_const_value(value: &ConstValue) -> Result<Duration, RuntimeError> {
///         let (secs, nanos) = rune::from_const_value::<(u64, u32)>(value)?;
///         Ok(Duration::new(secs, nanos))
///     }
///
///     #[inline]
///     pub(super) fn from_value(value: Value) -> Result<Duration, RuntimeError> {
///         let (secs, nanos) = rune::from_value::<(u64, u32)>(value)?;
///         Ok(Duration::new(secs, nanos))
///     }
/// }
///
/// #[rune::module(::time)]
/// pub fn module() -> Result<Module, ContextError> {
///     let mut m = Module::from_meta(module__meta)?;
///     m.ty::<Duration>()?;
///
///     m
///         .constant(
///             "SECOND",
///             Duration {
///                 inner: std::time::Duration::from_secs(1),
///             },
///         )
///         .build_associated::<Duration>()?
///         .docs(docstring! {
///             /// The duration of one second.
///             ///
///             /// # Examples
///             ///
///             /// ```rune
///             /// use time::Duration;
///             ///
///             /// let duration = Duration::SECOND;
///             /// ```
///         })?;
///
///     Ok(m)
/// }
/// ```
pub use rune_macros::ToConstValue;

use super::{AnyTypeInfo, RuntimeError, VmIntegerRepr};

/// Cheap conversion trait to convert something infallibly into a [`ConstValue`].
pub trait IntoConstValue {
    /// Convert into a dynamic [`ConstValue`].
    #[doc(hidden)]
    fn into_const_value(self) -> alloc::Result<ConstValue>;
}

impl IntoConstValue for ConstValue {
    #[inline]
    fn into_const_value(self) -> alloc::Result<ConstValue> {
        Ok(self)
    }
}

impl IntoConstValue for &ConstValue {
    #[inline]
    fn into_const_value(self) -> alloc::Result<ConstValue> {
        self.try_clone()
    }
}

/// Convert something into a [`ConstValue`].
///
/// # Examples
///
/// ```
/// let value = rune::to_const_value((i32::MIN, u64::MAX))?;
/// let (a, b) = rune::from_const_value::<(i32, u64)>(value)?;
///
/// assert_eq!(a, i32::MIN);
/// assert_eq!(b, u64::MAX);
/// # Ok::<_, rune::support::Error>(())
/// ```
pub fn from_const_value<T>(value: impl IntoConstValue) -> Result<T, RuntimeError>
where
    T: FromConstValue,
{
    T::from_const_value(value.into_const_value()?)
}

/// Convert something into a [`ConstValue`].
///
/// # Examples
///
/// ```
/// let value = rune::to_const_value((i32::MIN, u64::MAX))?;
/// let (a, b) = rune::from_const_value::<(i32, u64)>(value)?;
///
/// assert_eq!(a, i32::MIN);
/// assert_eq!(b, u64::MAX);
/// # Ok::<_, rune::support::Error>(())
/// ```
pub fn to_const_value(value: impl ToConstValue) -> Result<ConstValue, RuntimeError> {
    value.to_const_value()
}

/// Trait to perform a conversion to a [`ConstValue`].
pub trait ToConstValue: Sized {
    /// Convert into a constant value.
    fn to_const_value(self) -> Result<ConstValue, RuntimeError>;

    /// Return the constant constructor for the given type.
    #[inline]
    #[doc(hidden)]
    fn construct() -> Option<Arc<dyn ConstConstruct>> {
        None
    }
}

impl ToConstValue for ConstValue {
    #[inline]
    fn to_const_value(self) -> Result<ConstValue, RuntimeError> {
        Ok(self)
    }
}

impl ToConstValue for Value {
    #[inline]
    fn to_const_value(self) -> Result<ConstValue, RuntimeError> {
        ConstValue::from_value_ref(&self)
    }
}

#[derive(Debug, TryClone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "musli", derive(Decode, Encode))]
pub(crate) enum ConstValueKind {
    /// An inline constant value.
    Inline(#[try_clone(copy)] Inline),
    /// A string constant designated by its slot.
    String(String),
    /// A byte string.
    Bytes(Bytes),
    /// A vector of values.
    Vec(Vec<ConstValue>),
    /// An anonymous tuple.
    Tuple(Box<[ConstValue]>),
    /// An anonymous object.
    Object(HashMap<String, ConstValue>),
    /// An option.
    Option(Option<Box<ConstValue>>),
    /// A struct with the given type.
    Struct(Hash, Box<[ConstValue]>),
}

impl ConstValueKind {
    fn type_info(&self) -> TypeInfo {
        fn full_name(f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "constant struct")
        }

        match self {
            ConstValueKind::Inline(value) => value.type_info(),
            ConstValueKind::String(..) => TypeInfo::any::<String>(),
            ConstValueKind::Bytes(..) => TypeInfo::any::<Bytes>(),
            ConstValueKind::Vec(..) => TypeInfo::any::<runtime::Vec>(),
            ConstValueKind::Tuple(..) => TypeInfo::any::<OwnedTuple>(),
            ConstValueKind::Object(..) => TypeInfo::any::<Object>(),
            ConstValueKind::Option(..) => TypeInfo::any::<Option<Value>>(),
            ConstValueKind::Struct(hash, ..) => {
                TypeInfo::any_type_info(AnyTypeInfo::new(full_name, *hash))
            }
        }
    }
}

/// A constant value.
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize), serde(transparent))]
#[cfg_attr(feature = "musli", derive(Encode, Decode), musli(transparent))]
pub struct ConstValue {
    kind: ConstValueKind,
}

impl ConstValue {
    /// Construct a new tuple constant value.
    pub fn tuple(values: Box<[ConstValue]>) -> ConstValue {
        ConstValue {
            kind: ConstValueKind::Tuple(values),
        }
    }

    /// Construct a constant value for a struct.
    pub fn for_struct<const N: usize>(
        hash: Hash,
        fields: [ConstValue; N],
    ) -> Result<ConstValue, RuntimeError> {
        let fields = Box::<[ConstValue]>::try_from(fields)?;

        Ok(ConstValue {
            kind: ConstValueKind::Struct(hash, fields),
        })
    }

    /// Try to coerce the current value as the specified integer `T`.
    ///
    /// # Examples
    ///
    /// ```
    /// let value = rune::to_const_value(u32::MAX)?;
    ///
    /// assert_eq!(value.as_integer::<u64>()?, u32::MAX as u64);
    /// assert!(value.as_integer::<i32>().is_err());
    ///
    /// # Ok::<(), rune::support::Error>(())
    /// ```
    pub fn as_integer<T>(&self) -> Result<T, RuntimeError>
    where
        T: TryFrom<i64> + TryFrom<u64>,
    {
        match self.kind {
            ConstValueKind::Inline(Inline::Signed(value)) => match value.try_into() {
                Ok(number) => Ok(number),
                Err(..) => Err(RuntimeError::new(
                    VmErrorKind::ValueToIntegerCoercionError {
                        from: VmIntegerRepr::from(value),
                        to: any::type_name::<T>(),
                    },
                )),
            },
            ConstValueKind::Inline(Inline::Unsigned(value)) => match value.try_into() {
                Ok(number) => Ok(number),
                Err(..) => Err(RuntimeError::new(
                    VmErrorKind::ValueToIntegerCoercionError {
                        from: VmIntegerRepr::from(value),
                        to: any::type_name::<T>(),
                    },
                )),
            },
            ref kind => Err(RuntimeError::new(VmErrorKind::ExpectedNumber {
                actual: kind.type_info(),
            })),
        }
    }

    inline_macros!(inline_into);

    /// Coerce into tuple.
    #[inline]
    pub fn into_tuple(self) -> Result<Box<[ConstValue]>, RuntimeError> {
        match self.kind {
            ConstValueKind::Tuple(tuple) => Ok(tuple),
            kind => Err(RuntimeError::expected::<Tuple>(kind.type_info())),
        }
    }

    /// Access the interior value.
    pub(crate) fn as_kind(&self) -> &ConstValueKind {
        &self.kind
    }

    /// Construct a constant value from a reference to a value..
    pub(crate) fn from_value_ref(value: &Value) -> Result<ConstValue, RuntimeError> {
        let inner = match value.as_ref() {
            Repr::Inline(value) => ConstValueKind::Inline(*value),
            Repr::Dynamic(value) => {
                return Err(RuntimeError::from(VmErrorKind::ConstNotSupported {
                    actual: value.type_info(),
                }));
            }
            Repr::Any(value) => match value.type_hash() {
                Option::<Value>::HASH => {
                    let option = value.borrow_ref::<Option<Value>>()?;

                    ConstValueKind::Option(match &*option {
                        Some(some) => Some(Box::try_new(Self::from_value_ref(some)?)?),
                        None => None,
                    })
                }
                String::HASH => {
                    let s = value.borrow_ref::<String>()?;
                    ConstValueKind::String(s.try_to_owned()?)
                }
                Bytes::HASH => {
                    let s = value.borrow_ref::<Bytes>()?;
                    ConstValueKind::Bytes(s.try_to_owned()?)
                }
                runtime::Vec::HASH => {
                    let vec = value.borrow_ref::<runtime::Vec>()?;
                    let mut const_vec = Vec::try_with_capacity(vec.len())?;

                    for value in vec.iter() {
                        const_vec.try_push(Self::from_value_ref(value)?)?;
                    }

                    ConstValueKind::Vec(const_vec)
                }
                runtime::OwnedTuple::HASH => {
                    let tuple = value.borrow_ref::<runtime::OwnedTuple>()?;
                    let mut const_tuple = Vec::try_with_capacity(tuple.len())?;

                    for value in tuple.iter() {
                        const_tuple.try_push(Self::from_value_ref(value)?)?;
                    }

                    ConstValueKind::Tuple(const_tuple.try_into_boxed_slice()?)
                }
                Object::HASH => {
                    let object = value.borrow_ref::<Object>()?;
                    let mut const_object = HashMap::try_with_capacity(object.len())?;

                    for (key, value) in object.iter() {
                        let key = key.try_clone()?;
                        let value = Self::from_value_ref(value)?;
                        const_object.try_insert(key, value)?;
                    }

                    ConstValueKind::Object(const_object)
                }
                _ => {
                    return Err(RuntimeError::from(VmErrorKind::ConstNotSupported {
                        actual: value.type_info(),
                    }));
                }
            },
        };

        Ok(Self { kind: inner })
    }

    #[inline]
    #[cfg(test)]
    pub(crate) fn to_value(&self) -> Result<Value, RuntimeError> {
        self.to_value_with(&EmptyConstContext)
    }

    /// Convert into virtual machine value.
    ///
    /// We provide this associated method since a constant value can be
    /// converted into a value infallibly, which is not captured by the trait
    /// otherwise.
    pub(crate) fn to_value_with(&self, cx: &dyn ConstContext) -> Result<Value, RuntimeError> {
        match &self.kind {
            ConstValueKind::Inline(value) => Ok(Value::from(*value)),
            ConstValueKind::String(string) => Ok(Value::try_from(string.try_clone()?)?),
            ConstValueKind::Bytes(b) => Ok(Value::try_from(b.try_clone()?)?),
            ConstValueKind::Option(option) => Ok(Value::try_from(match option {
                Some(some) => Some(Self::to_value_with(some, cx)?),
                None => None,
            })?),
            ConstValueKind::Vec(vec) => {
                let mut v = runtime::Vec::with_capacity(vec.len())?;

                for value in vec {
                    v.push(Self::to_value_with(value, cx)?)?;
                }

                Ok(Value::try_from(v)?)
            }
            ConstValueKind::Tuple(tuple) => {
                let mut t = Vec::try_with_capacity(tuple.len())?;

                for value in tuple.iter() {
                    t.try_push(Self::to_value_with(value, cx)?)?;
                }

                Ok(Value::try_from(OwnedTuple::try_from(t)?)?)
            }
            ConstValueKind::Object(object) => {
                let mut o = Object::with_capacity(object.len())?;

                for (key, value) in object {
                    let key = key.try_clone()?;
                    let value = Self::to_value_with(value, cx)?;
                    o.insert(key, value)?;
                }

                Ok(Value::try_from(o)?)
            }
            ConstValueKind::Struct(hash, fields) => {
                let Some(constructor) = cx.get(*hash) else {
                    return Err(RuntimeError::missing_constant_constructor(*hash));
                };

                constructor.const_construct(fields)
            }
        }
    }

    /// Get the type information of the value.
    pub(crate) fn type_info(&self) -> TypeInfo {
        self.kind.type_info()
    }
}

impl TryClone for ConstValue {
    fn try_clone(&self) -> alloc::Result<Self> {
        Ok(Self {
            kind: self.kind.try_clone()?,
        })
    }
}

impl FromValue for ConstValue {
    #[inline]
    fn from_value(value: Value) -> Result<Self, RuntimeError> {
        ConstValue::from_value_ref(&value)
    }
}

impl ToValue for ConstValue {
    #[inline]
    fn to_value(self) -> Result<Value, RuntimeError> {
        ConstValue::to_value_with(&self, &EmptyConstContext)
    }
}

impl From<ConstValueKind> for ConstValue {
    #[inline]
    fn from(kind: ConstValueKind) -> Self {
        Self { kind }
    }
}

impl From<Inline> for ConstValue {
    #[inline]
    fn from(value: Inline) -> Self {
        Self::from(ConstValueKind::Inline(value))
    }
}

impl From<String> for ConstValue {
    #[inline]
    fn from(value: String) -> Self {
        Self::from(ConstValueKind::String(value))
    }
}

impl From<Bytes> for ConstValue {
    #[inline]
    fn from(value: Bytes) -> Self {
        Self::from(ConstValueKind::Bytes(value))
    }
}

impl TryFrom<&str> for ConstValue {
    type Error = alloc::Error;

    #[inline]
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(ConstValue::from(String::try_from(value)?))
    }
}

impl ToConstValue for &str {
    #[inline]
    fn to_const_value(self) -> Result<ConstValue, RuntimeError> {
        Ok(ConstValue::try_from(self)?)
    }
}

impl TryFrom<&[u8]> for ConstValue {
    type Error = alloc::Error;

    #[inline]
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Ok(ConstValue::from(Bytes::try_from(value)?))
    }
}

impl ToConstValue for &[u8] {
    #[inline]
    fn to_const_value(self) -> Result<ConstValue, RuntimeError> {
        Ok(ConstValue::try_from(self)?)
    }
}

impl fmt::Debug for ConstValue {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.kind.fmt(f)
    }
}

/// Trait to perform a conversion from a [`ConstValue`].
pub trait FromConstValue: Sized {
    /// Convert from a constant value.
    fn from_const_value(value: ConstValue) -> Result<Self, RuntimeError>;
}

impl FromConstValue for ConstValue {
    #[inline]
    fn from_const_value(value: ConstValue) -> Result<Self, RuntimeError> {
        Ok(value)
    }
}

/// Implementation of a constant constructor.
///
/// Do not implement manually, this is provided when deriving [`ToConstValue`].
///
/// [`ToConstValue`]: derive@ToConstValue
pub trait ConstConstruct: 'static + Send + Sync {
    /// Construct from values.
    #[doc(hidden)]
    fn const_construct(&self, fields: &[ConstValue]) -> Result<Value, RuntimeError>;

    /// Construct from values.
    #[doc(hidden)]
    fn runtime_construct(&self, fields: &mut [Value]) -> Result<Value, RuntimeError>;
}

pub(crate) trait ConstContext {
    fn get(&self, hash: Hash) -> Option<&dyn ConstConstruct>;
}

pub(crate) struct EmptyConstContext;

impl ConstContext for EmptyConstContext {
    #[inline]
    fn get(&self, _: Hash) -> Option<&dyn ConstConstruct> {
        None
    }
}
