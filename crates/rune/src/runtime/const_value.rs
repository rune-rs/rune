use core::fmt;
use rust_alloc::sync::Arc;

use serde::{Deserialize, Serialize};

use crate as rune;
use crate::alloc::prelude::*;
use crate::alloc::{self, HashMap};
use crate::runtime::{
    self, BorrowRefRepr, Bytes, FromValue, Inline, Mutable, Object, OwnedTuple, RawStr, ToValue,
    TypeInfo, Value, VmErrorKind, VmResult,
};
use crate::{Hash, TypeHash};

/// Derive for the [`ToConstValue`](trait@ToConstValue) trait.
pub use rune_macros::ToConstValue;

use super::{AnyTypeInfo, RuntimeError};

/// Implementation of a constant constructor.
///
/// Do not implement manually, this is provided when deriving
/// [`ToConstValue`](derive@ToConstValue).
pub trait ConstConstruct {
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

/// A constant value.
#[derive(Deserialize, Serialize)]
#[serde(transparent)]
pub struct ConstValue {
    kind: ConstValueKind,
}

impl ConstValue {
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

    /// Convert into virtual machine value.
    ///
    /// We provide this associated method since a constant value can be
    /// converted into a value infallibly, which is not captured by the trait
    /// otherwise.
    pub(crate) fn to_value(&self, cx: &dyn ConstContext) -> Result<Value, RuntimeError> {
        match &self.kind {
            ConstValueKind::Inline(value) => Ok(Value::from(*value)),
            ConstValueKind::String(string) => Ok(Value::try_from(string.try_clone()?)?),
            ConstValueKind::Bytes(b) => Ok(Value::try_from(b.try_clone()?)?),
            ConstValueKind::Option(option) => Ok(Value::try_from(match option {
                Some(some) => Some(Self::to_value(some, cx)?),
                None => None,
            })?),
            ConstValueKind::Vec(vec) => {
                let mut v = runtime::Vec::with_capacity(vec.len())?;

                for value in vec {
                    v.push(Self::to_value(value, cx)?)?;
                }

                Ok(Value::try_from(v)?)
            }
            ConstValueKind::Tuple(tuple) => {
                let mut t = Vec::try_with_capacity(tuple.len())?;

                for value in tuple.iter() {
                    t.try_push(Self::to_value(value, cx)?)?;
                }

                Ok(Value::try_from(OwnedTuple::try_from(t)?)?)
            }
            ConstValueKind::Object(object) => {
                let mut o = Object::with_capacity(object.len())?;

                for (key, value) in object {
                    let key = key.try_clone()?;
                    let value = Self::to_value(value, cx)?;
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

    /// Try to coerce into boolean.
    pub fn as_bool(&self) -> Option<bool> {
        match self.kind {
            ConstValueKind::Inline(Inline::Bool(value)) => Some(value),
            _ => None,
        }
    }

    /// Try to coerce into an integer.
    pub fn as_i64(&self) -> Option<i64> {
        match self.kind {
            ConstValueKind::Inline(Inline::Integer(value)) => Some(value),
            _ => None,
        }
    }

    /// Get the type information of the value.
    pub(crate) fn type_info(&self) -> TypeInfo {
        match &self.kind {
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

impl TryClone for ConstValue {
    fn try_clone(&self) -> alloc::Result<Self> {
        Ok(Self {
            kind: self.kind.try_clone()?,
        })
    }
}

impl FromValue for ConstValue {
    #[inline]
    fn from_value(value: Value) -> VmResult<Self> {
        VmResult::Ok(vm_try!(ConstValue::from_value_ref(&value)))
    }
}

impl ToValue for ConstValue {
    #[inline]
    fn to_value(self) -> VmResult<Value> {
        VmResult::Ok(vm_try!(ConstValue::to_value(&self, &EmptyConstContext)))
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
