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

use crate::alloc;
use crate::alloc::prelude::*;
use crate::runtime;
use crate::{self as rune};
use crate::{hash_in, Hash, TypeHash};

use super::{
    AnyTypeInfo, Bytes, ExpectedType, FromValue, Inline, Object, OwnedTuple, Repr, RuntimeError,
    ToValue, Tuple, Type, TypeInfo, Value, VmErrorKind, VmIntegerRepr,
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

/// A dynamic constant value.
#[derive(Debug, TryClone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "musli", derive(Decode, Encode))]
pub(crate) struct ConstInstance {
    /// The type hash of the value.
    ///
    /// If the value is a variant, this is the type hash of the enum.
    #[try_clone(copy)]
    pub(crate) hash: Hash,
    /// The type hash of the variant.
    ///
    /// If this is not an enum, this is `Hash::EMPTY`.
    #[try_clone(copy)]
    pub(crate) variant_hash: Hash,
    /// The fields the value is constituted of.
    pub(crate) fields: Box<[ConstValue]>,
}

impl ConstInstance {
    fn type_info(&self) -> TypeInfo {
        fn struct_name(f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "unknown constant struct")
        }

        fn variant_name(f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "unknown constant variant")
        }

        match self.hash {
            Option::<Value>::HASH => TypeInfo::any::<Option<Value>>(),
            runtime::Vec::HASH => TypeInfo::any::<runtime::Vec>(),
            OwnedTuple::HASH => TypeInfo::any::<OwnedTuple>(),
            Object::HASH => TypeInfo::any::<Object>(),
            hash if self.variant_hash == Hash::EMPTY => {
                TypeInfo::any_type_info(AnyTypeInfo::new(struct_name, hash))
            }
            hash => TypeInfo::any_type_info(AnyTypeInfo::new(variant_name, hash)),
        }
    }
}

#[derive(Debug, TryClone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "musli", derive(Decode, Encode))]
pub(crate) enum ConstValueKind {
    /// An inline constant value.
    Inline(#[try_clone(copy)] Inline),
    /// A string constant designated by its slot.
    String(Box<str>),
    /// A byte string.
    Bytes(Box<[u8]>),
    /// An instance of some type of value.
    Instance(Box<ConstInstance>),
}

impl ConstValueKind {
    #[inline]
    fn type_info(&self) -> TypeInfo {
        match self {
            ConstValueKind::Inline(value) => value.type_info(),
            ConstValueKind::String(..) => TypeInfo::any::<String>(),
            ConstValueKind::Bytes(..) => TypeInfo::any::<Bytes>(),
            ConstValueKind::Instance(instance) => instance.type_info(),
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
    /// Construct a constant value that is a string.
    pub fn string(value: impl AsRef<str>) -> Result<ConstValue, RuntimeError> {
        let value = Box::try_from(value.as_ref())?;
        Ok(Self::from(ConstValueKind::String(value)))
    }

    /// Construct a constant value that is bytes.
    pub fn bytes(value: impl AsRef<[u8]>) -> Result<ConstValue, RuntimeError> {
        let value = Box::try_from(value.as_ref())?;
        Ok(Self::from(ConstValueKind::Bytes(value)))
    }

    /// Construct a new tuple constant value.
    pub fn tuple(fields: Box<[ConstValue]>) -> Result<ConstValue, RuntimeError> {
        let instance = ConstInstance {
            hash: OwnedTuple::HASH,
            variant_hash: Hash::EMPTY,
            fields,
        };

        let instance = Box::try_new(instance)?;
        Ok(Self::from(ConstValueKind::Instance(instance)))
    }

    /// Construct a constant value for a struct.
    pub fn for_struct<const N: usize>(
        hash: Hash,
        fields: [ConstValue; N],
    ) -> Result<ConstValue, RuntimeError> {
        let fields = Box::<[ConstValue]>::try_from(fields)?;

        let instance = ConstInstance {
            hash,
            variant_hash: Hash::EMPTY,
            fields,
        };

        let instance = Box::try_new(instance)?;
        Ok(Self::from(ConstValueKind::Instance(instance)))
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
    pub fn into_tuple(self) -> Result<Box<[ConstValue]>, ExpectedType> {
        match self.kind {
            ConstValueKind::Instance(instance) => {
                let instance = Box::into_inner(instance);

                match instance.hash {
                    OwnedTuple::HASH => Ok(instance.fields),
                    _ => Err(ExpectedType::new::<Tuple>(instance.type_info())),
                }
            }
            kind => Err(ExpectedType::new::<Tuple>(kind.type_info())),
        }
    }

    /// Access the interior value.
    pub(crate) fn as_kind(&self) -> &ConstValueKind {
        &self.kind
    }

    /// Construct a constant value from a reference to a value..
    pub(crate) fn from_value_ref(value: &Value) -> Result<ConstValue, RuntimeError> {
        let kind = match value.as_ref() {
            Repr::Inline(value) => ConstValueKind::Inline(*value),
            Repr::Dynamic(value) => {
                return Err(RuntimeError::from(VmErrorKind::ConstNotSupported {
                    actual: value.type_info(),
                }));
            }
            Repr::Any(value) => match value.type_hash() {
                String::HASH => {
                    return ConstValue::string(value.borrow_ref::<String>()?.as_str());
                }
                Bytes::HASH => {
                    return ConstValue::bytes(value.borrow_ref::<Bytes>()?.as_slice());
                }
                runtime::OwnedTuple::HASH => {
                    let tuple = value.borrow_ref::<runtime::OwnedTuple>()?;
                    let mut const_tuple = Vec::try_with_capacity(tuple.len())?;

                    for value in tuple.iter() {
                        const_tuple.try_push(Self::from_value_ref(value)?)?;
                    }

                    return ConstValue::tuple(const_tuple.try_into_boxed_slice()?);
                }
                Object::HASH => {
                    let object = value.borrow_ref::<Object>()?;
                    let mut fields = Vec::try_with_capacity(object.len())?;

                    for (key, value) in object.iter() {
                        let key = ConstValue::string(key.as_str())?;
                        let value = Self::from_value_ref(value)?;
                        fields.try_push(ConstValue::tuple(Box::try_from([key, value])?)?)?;
                    }

                    let instance = ConstInstance {
                        hash: Object::HASH,
                        variant_hash: Hash::EMPTY,
                        fields: fields.try_into_boxed_slice()?,
                    };

                    let instance = Box::try_new(instance)?;
                    ConstValueKind::Instance(instance)
                }
                Option::<Value>::HASH => {
                    let option = value.borrow_ref::<Option<Value>>()?;

                    let (variant_hash, fields) = match &*option {
                        Some(some) => (
                            hash_in!(crate, ::std::option::Option::Some),
                            Box::try_from([Self::from_value_ref(some)?])?,
                        ),
                        None => (hash_in!(crate, ::std::option::Option::None), Box::default()),
                    };

                    let instance = ConstInstance {
                        hash: Option::<Value>::HASH,
                        variant_hash,
                        fields,
                    };
                    ConstValueKind::Instance(Box::try_new(instance)?)
                }
                runtime::Vec::HASH => {
                    let vec = value.borrow_ref::<runtime::Vec>()?;
                    let mut const_vec = Vec::try_with_capacity(vec.len())?;

                    for value in vec.iter() {
                        const_vec.try_push(Self::from_value_ref(value)?)?;
                    }

                    let fields = Box::try_from(const_vec)?;

                    let instance = ConstInstance {
                        hash: runtime::Vec::HASH,
                        variant_hash: Hash::EMPTY,
                        fields,
                    };
                    ConstValueKind::Instance(Box::try_new(instance)?)
                }
                _ => {
                    return Err(RuntimeError::from(VmErrorKind::ConstNotSupported {
                        actual: value.type_info(),
                    }));
                }
            },
        };

        Ok(Self { kind })
    }

    #[inline]
    #[cfg(test)]
    pub(crate) fn to_value(&self) -> Result<Value, RuntimeError> {
        self.to_value_with(&EmptyConstContext)
    }

    /// Convert into a pair of tuples.
    pub(crate) fn as_string(&self) -> Result<&str, ExpectedType> {
        let ConstValueKind::String(value) = &self.kind else {
            return Err(ExpectedType::new::<String>(self.kind.type_info()));
        };

        Ok(value)
    }

    /// Convert into a pair of tuples.
    pub(crate) fn as_pair(&self) -> Result<(&ConstValue, &ConstValue), ExpectedType> {
        let ConstValueKind::Instance(instance) = &self.kind else {
            return Err(ExpectedType::new::<Tuple>(self.kind.type_info()));
        };

        if !matches!(instance.hash, OwnedTuple::HASH) {
            return Err(ExpectedType::new::<Tuple>(instance.type_info()));
        }

        let [a, b] = instance.fields.as_ref() else {
            return Err(ExpectedType::new::<Tuple>(instance.type_info()));
        };

        Ok((a, b))
    }

    /// Convert into virtual machine value.
    ///
    /// We provide this associated method since a constant value can be
    /// converted into a value infallibly, which is not captured by the trait
    /// otherwise.
    pub(crate) fn to_value_with(&self, cx: &dyn ConstContext) -> Result<Value, RuntimeError> {
        match &self.kind {
            ConstValueKind::Inline(value) => Ok(Value::from(*value)),
            ConstValueKind::String(string) => Ok(Value::try_from(string.as_ref())?),
            ConstValueKind::Bytes(b) => Ok(Value::try_from(b.as_ref())?),
            ConstValueKind::Instance(instance) => match &**instance {
                ConstInstance {
                    hash,
                    variant_hash: Hash::EMPTY,
                    fields,
                } => match *hash {
                    runtime::OwnedTuple::HASH => {
                        let mut t = Vec::try_with_capacity(fields.len())?;

                        for value in fields.iter() {
                            t.try_push(Self::to_value_with(value, cx)?)?;
                        }

                        Ok(Value::try_from(OwnedTuple::try_from(t)?)?)
                    }
                    runtime::Vec::HASH => {
                        let mut v = runtime::Vec::with_capacity(fields.len())?;

                        for value in fields.iter() {
                            v.push(Self::to_value_with(value, cx)?)?;
                        }

                        Ok(Value::try_from(v)?)
                    }
                    runtime::Object::HASH => {
                        let mut o = Object::with_capacity(fields.len())?;

                        for value in fields.iter() {
                            let (key, value) = value.as_pair()?;
                            let key = key.as_string()?.try_to_string()?;
                            let value = Self::to_value_with(value, cx)?;
                            o.insert(key, value)?;
                        }

                        Ok(Value::try_from(o)?)
                    }
                    _ => {
                        let Some(constructor) = cx.get(*hash) else {
                            return Err(RuntimeError::missing_constant_constructor(*hash));
                        };

                        constructor.const_construct(fields)
                    }
                },
                ConstInstance {
                    hash,
                    variant_hash,
                    fields,
                } => {
                    match (*variant_hash, &fields[..]) {
                        // If the hash is `Option`, we can return a value directly.
                        (hash_in!(crate, ::std::option::Option::Some), [value]) => {
                            let value = Self::to_value_with(value, cx)?;
                            Ok(Value::try_from(Some(value))?)
                        }
                        (hash_in!(crate, ::std::option::Option::None), []) => {
                            Ok(Value::try_from(None)?)
                        }
                        _ => Err(RuntimeError::missing_constant_constructor(*hash)),
                    }
                }
            },
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

impl TryFrom<String> for ConstValue {
    type Error = alloc::Error;

    #[inline]
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Ok(Self::from(Box::<str>::try_from(value)?))
    }
}

impl From<Box<str>> for ConstValue {
    #[inline]
    fn from(value: Box<str>) -> Self {
        Self::from(ConstValueKind::String(value))
    }
}

impl TryFrom<Bytes> for ConstValue {
    type Error = alloc::Error;

    #[inline]
    fn try_from(value: Bytes) -> Result<Self, Self::Error> {
        Self::try_from(value.as_slice())
    }
}

impl TryFrom<&str> for ConstValue {
    type Error = alloc::Error;

    #[inline]
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(ConstValue::from(Box::<str>::try_from(value)?))
    }
}

impl ToConstValue for &str {
    #[inline]
    fn to_const_value(self) -> Result<ConstValue, RuntimeError> {
        Ok(ConstValue::try_from(self)?)
    }
}

impl From<Box<[u8]>> for ConstValue {
    #[inline]
    fn from(value: Box<[u8]>) -> Self {
        Self::from(ConstValueKind::Bytes(value))
    }
}

impl TryFrom<&[u8]> for ConstValue {
    type Error = alloc::Error;

    #[inline]
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Ok(Self::from(Box::<[u8]>::try_from(value)?))
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

impl FromConstValue for bool {
    #[inline]
    fn from_const_value(value: ConstValue) -> Result<Self, RuntimeError> {
        value.as_bool()
    }
}

impl FromConstValue for char {
    #[inline]
    fn from_const_value(value: ConstValue) -> Result<Self, RuntimeError> {
        value.as_char()
    }
}

macro_rules! impl_integer {
    ($($ty:ty),* $(,)?) => {
        $(
            impl FromConstValue for $ty {
                #[inline]
                fn from_const_value(value: ConstValue) -> Result<Self, RuntimeError> {
                    value.as_integer()
                }
            }
        )*
    };
}

impl_integer!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);

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
