use crate::{
    Bytes, FromValue, Shared, StaticString, ToValue, Tuple, TypeInfo, Value, Vec, VmError,
    VmErrorKind,
};
use serde::{de, ser};
use std::fmt;
use std::sync::Arc;
use std::vec;

/// A key that can be used as an anonymous object key.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Key {
    /// A constant unit.
    Unit,
    /// A byte.
    Byte(u8),
    /// A character.
    Char(char),
    /// A boolean constant value.
    Bool(bool),
    /// An integer constant.
    Integer(i64),
    /// A string constant designated by its slot.
    String(String),
    /// A static string.
    StaticString(Arc<StaticString>),
    /// A byte string.
    Bytes(Bytes),
    /// A vector of values.
    Vec(vec::Vec<Key>),
    /// An anonymous tuple.
    Tuple(Box<[Key]>),
    /// An option.
    Option(Option<Box<Key>>),
}

impl Key {
    /// Convert into virtual machine value.
    ///
    /// We provide this associated method since a constant value can be
    /// converted into a value infallibly, which is not captured by the trait
    /// otherwise.
    pub fn into_value(self) -> Value {
        match self {
            Self::Unit => Value::Unit,
            Self::Byte(b) => Value::Byte(b),
            Self::Char(c) => Value::Char(c),
            Self::Bool(b) => Value::Bool(b),
            Self::Integer(n) => Value::Integer(n),
            Self::String(s) => Value::String(Shared::new(s)),
            Self::StaticString(s) => Value::StaticString(s),
            Self::Bytes(b) => Value::Bytes(Shared::new(b)),
            Self::Option(option) => Value::Option(Shared::new(match option {
                Some(some) => Some(some.into_value()),
                None => None,
            })),
            Self::Vec(vec) => {
                let mut v = Vec::with_capacity(vec.len());

                for value in vec {
                    v.push(value.into_value());
                }

                Value::Vec(Shared::new(v))
            }
            Self::Tuple(tuple) => {
                let mut t = vec::Vec::with_capacity(tuple.len());

                for value in vec::Vec::from(tuple) {
                    t.push(value.into_value());
                }

                Value::Tuple(Shared::new(Tuple::from(t)))
            }
        }
    }

    /// Try to coerce into boolean.
    pub fn into_bool(self) -> Result<bool, Self> {
        match self {
            Self::Bool(value) => Ok(value),
            value => Err(value),
        }
    }

    /// Get the type information of the value.
    pub fn type_info(&self) -> TypeInfo {
        match self {
            Self::Unit => TypeInfo::StaticType(crate::UNIT_TYPE),
            Self::Byte(..) => TypeInfo::StaticType(crate::BYTE_TYPE),
            Self::Char(..) => TypeInfo::StaticType(crate::CHAR_TYPE),
            Self::Bool(..) => TypeInfo::StaticType(crate::BOOL_TYPE),
            Self::String(..) => TypeInfo::StaticType(crate::STRING_TYPE),
            Self::StaticString(..) => TypeInfo::StaticType(crate::STRING_TYPE),
            Self::Bytes(..) => TypeInfo::StaticType(crate::BYTES_TYPE),
            Self::Integer(..) => TypeInfo::StaticType(crate::INTEGER_TYPE),
            Self::Vec(..) => TypeInfo::StaticType(crate::VEC_TYPE),
            Self::Tuple(..) => TypeInfo::StaticType(crate::TUPLE_TYPE),
            Self::Option(..) => TypeInfo::StaticType(crate::OPTION_TYPE),
        }
    }
}

impl FromValue for Key {
    fn from_value(value: Value) -> Result<Self, VmError> {
        Ok(match value {
            Value::Unit => Self::Unit,
            Value::Byte(b) => Self::Byte(b),
            Value::Char(c) => Self::Char(c),
            Value::Bool(b) => Self::Bool(b),
            Value::Integer(n) => Self::Integer(n),
            Value::String(s) => {
                let s = s.take()?;
                Self::String(s)
            }
            Value::StaticString(s) => Self::StaticString(s),
            Value::Option(option) => Self::Option(match option.take()? {
                Some(some) => Some(Box::new(Self::from_value(some)?)),
                None => None,
            }),
            Value::Bytes(b) => {
                let b = b.take()?;
                Self::Bytes(Bytes::from(b))
            }
            Value::Vec(vec) => {
                let vec = vec.take()?;
                let mut const_vec = vec::Vec::with_capacity(vec.len());

                for value in vec {
                    const_vec.push(Self::from_value(value)?);
                }

                Self::Vec(const_vec)
            }
            Value::Tuple(tuple) => {
                let tuple = tuple.take()?;
                let mut const_tuple = vec::Vec::with_capacity(tuple.len());

                for value in vec::Vec::from(tuple.into_inner()) {
                    const_tuple.push(Self::from_value(value)?);
                }

                Self::Tuple(const_tuple.into_boxed_slice())
            }
            value => {
                return Err(VmError::from(VmErrorKind::KeyNotSupported {
                    actual: value.type_info()?,
                }))
            }
        })
    }
}

impl ToValue for Key {
    fn to_value(self) -> Result<Value, VmError> {
        Ok(Key::into_value(self))
    }
}

/// Deserialize implementation for value.
impl<'de> de::Deserialize<'de> for Key {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_any(KeyVisitor)
    }
}

/// Serialize implementation for value.
impl ser::Serialize for Key {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        use serde::ser::SerializeSeq as _;

        match self {
            Self::Unit => serializer.serialize_unit(),
            Self::Bool(b) => serializer.serialize_bool(*b),
            Self::Char(c) => serializer.serialize_char(*c),
            Self::Byte(c) => serializer.serialize_u8(*c),
            Self::Integer(integer) => serializer.serialize_i64(*integer),
            Self::StaticString(string) => serializer.serialize_str(string.as_ref()),
            Self::String(string) => serializer.serialize_str(&*string),
            Self::Bytes(bytes) => serializer.serialize_bytes(&*bytes),
            Self::Vec(vec) => {
                let mut serializer = serializer.serialize_seq(Some(vec.len()))?;

                for value in &*vec {
                    serializer.serialize_element(value)?;
                }

                serializer.end()
            }
            Self::Tuple(tuple) => {
                let mut serializer = serializer.serialize_seq(Some(tuple.len()))?;

                for value in tuple.iter() {
                    serializer.serialize_element(value)?;
                }

                serializer.end()
            }
            Self::Option(option) => <Option<Box<Key>>>::serialize(option, serializer),
        }
    }
}

struct KeyVisitor;

impl<'de> de::Visitor<'de> for KeyVisitor {
    type Value = Key;

    fn expecting(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.write_str("any valid key")
    }

    #[inline]
    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Key::String(value.to_owned()))
    }

    #[inline]
    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Key::String(value))
    }

    #[inline]
    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Key::Bytes(Bytes::from_vec(v.to_vec())))
    }

    #[inline]
    fn visit_byte_buf<E>(self, v: vec::Vec<u8>) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Key::Bytes(Bytes::from_vec(v)))
    }

    #[inline]
    fn visit_i8<E>(self, v: i8) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Key::Integer(v as i64))
    }

    #[inline]
    fn visit_i16<E>(self, v: i16) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Key::Integer(v as i64))
    }

    #[inline]
    fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Key::Integer(v as i64))
    }

    #[inline]
    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Key::Integer(v))
    }

    #[inline]
    fn visit_i128<E>(self, v: i128) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Key::Integer(v as i64))
    }

    #[inline]
    fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Key::Integer(v as i64))
    }

    #[inline]
    fn visit_u16<E>(self, v: u16) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Key::Integer(v as i64))
    }

    #[inline]
    fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Key::Integer(v as i64))
    }

    #[inline]
    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Key::Integer(v as i64))
    }

    #[inline]
    fn visit_u128<E>(self, v: u128) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Key::Integer(v as i64))
    }

    #[inline]
    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Key::Bool(v))
    }

    #[inline]
    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Key::Unit)
    }

    #[inline]
    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Key::Unit)
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

        Ok(Key::Vec(vec))
    }
}
