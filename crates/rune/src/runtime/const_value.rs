#[macro_use]
mod macros;

use core::any;
use core::cmp::Ordering;
use core::fmt;

use rust_alloc::sync::Arc;

use serde::{Deserialize, Serialize};

use crate as rune;
use crate::alloc::prelude::*;
use crate::alloc::{self, HashMap};
use crate::{Hash, TypeHash};

use super::{
    BorrowRefRepr, Bytes, FromValue, Inline, Mutable, Object, OwnedTuple, RawStr, ToValue, Tuple,
    Type, TypeInfo, Value, VmErrorKind,
};

/// Derive for the [`ToConstValue`](trait@ToConstValue) trait.
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
/// let value = rune::to_const_value((1u32, 2u64))?;
/// let (a, b) = rune::from_const_value::<(1u32, 2u64)>(value)?;
///
/// assert_eq!(a, 1);
/// assert_eq!(b, 2);
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
/// let value = rune::to_const_value((1u32, 2u64))?;
/// let (a, b) = rune::from_const_value::<(1u32, 2u64)>(value)?;
///
/// assert_eq!(a, 1);
/// assert_eq!(b, 2);
/// # Ok::<_, rune::support::Error>(())
/// ```
pub fn to_const_value(value: impl ToConstValue) -> Result<ConstValue, RuntimeError> {
    value.to_const_value()
}

/// Convert a value into a constant value.
pub trait ToConstValue: Sized {
    /// Convert into a constant value.
    fn to_const_value(self) -> Result<ConstValue, RuntimeError>;

    /// Return the constant constructor for the given type.
    #[inline]
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

#[derive(Debug, TryClone, Deserialize, Serialize)]
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
        match self {
            ConstValueKind::Inline(value) => value.type_info(),
            ConstValueKind::String(..) => {
                TypeInfo::static_type(crate::runtime::static_type::STRING)
            }
            ConstValueKind::Bytes(..) => TypeInfo::static_type(crate::runtime::static_type::BYTES),
            ConstValueKind::Vec(..) => TypeInfo::static_type(crate::runtime::static_type::VEC),
            ConstValueKind::Tuple(..) => TypeInfo::static_type(crate::runtime::static_type::TUPLE),
            ConstValueKind::Object(..) => {
                TypeInfo::static_type(crate::runtime::static_type::OBJECT)
            }
            ConstValueKind::Option(..) => {
                TypeInfo::static_type(crate::runtime::static_type::OPTION)
            }
            ConstValueKind::Struct(hash, ..) => TypeInfo::any_type_info(AnyTypeInfo::new(
                RawStr::from_str("constant struct"),
                *hash,
            )),
        }
    }
}

/// A constant value.
#[derive(Deserialize, Serialize)]
#[serde(transparent)]
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
    /// assert_eq!(value.try_as_integer::<u64>()?, u32::MAX as u64);
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
        let inner = match value.borrow_ref_repr()? {
            BorrowRefRepr::Inline(value) => ConstValueKind::Inline(*value),
            BorrowRefRepr::Mutable(value) => match &*value {
                Mutable::Option(option) => ConstValueKind::Option(match option {
                    Some(some) => Some(Box::try_new(Self::from_value_ref(some)?)?),
                    None => None,
                }),
                Mutable::Vec(ref vec) => {
                    let mut const_vec = Vec::try_with_capacity(vec.len())?;

                    for value in vec {
                        const_vec.try_push(Self::from_value_ref(value)?)?;
                    }

                    ConstValueKind::Vec(const_vec)
                }
                Mutable::Tuple(ref tuple) => {
                    let mut const_tuple = Vec::try_with_capacity(tuple.len())?;

                    for value in tuple.iter() {
                        const_tuple.try_push(Self::from_value_ref(value)?)?;
                    }

                    ConstValueKind::Tuple(const_tuple.try_into_boxed_slice()?)
                }
                Mutable::Object(ref object) => {
                    let mut const_object = HashMap::try_with_capacity(object.len())?;

                    for (key, value) in object {
                        let key = key.try_clone()?;
                        let value = Self::from_value_ref(value)?;
                        const_object.try_insert(key, value)?;
                    }

                    ConstValueKind::Object(const_object)
                }
                value => {
                    return Err(RuntimeError::from(VmErrorKind::ConstNotSupported {
                        actual: value.type_info(),
                    }))
                }
            },
            BorrowRefRepr::Any(value) => match value.type_hash() {
                String::HASH => {
                    let s = value.borrow_ref::<String>()?;
                    ConstValueKind::String(s.try_to_owned()?)
                }
                Bytes::HASH => {
                    let s = value.borrow_ref::<Bytes>()?;
                    ConstValueKind::Bytes(s.try_to_owned()?)
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
                let mut v = super::Vec::with_capacity(vec.len())?;

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

/// Convert a value from a constant value.
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
/// Do not implement manually, this is provided when deriving
/// [`ToConstValue`](derive@ToConstValue).
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
