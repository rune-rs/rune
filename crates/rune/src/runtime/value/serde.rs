use core::fmt;

use crate::alloc;
use crate::alloc::prelude::*;
use crate::runtime::{Bytes, Object, Shared, Vec};

use serde::de::{self, Deserialize as _, Error};
use serde::ser::{self, SerializeMap as _, SerializeSeq as _};

use super::Value;

/// Deserialize implementation for value pointers.
impl<'de> de::Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_any(VmVisitor)
    }
}

/// Serialize implementation for value pointers.
impl ser::Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        match self {
            Value::Bool(b) => serializer.serialize_bool(*b),
            Value::Char(c) => serializer.serialize_char(*c),
            Value::Byte(c) => serializer.serialize_u8(*c),
            Value::Integer(integer) => serializer.serialize_i64(*integer),
            Value::Float(float) => serializer.serialize_f64(*float),
            Value::Type(..) => Err(ser::Error::custom("cannot serialize types")),
            Value::Ordering(..) => Err(ser::Error::custom("cannot serialize orderings")),
            Value::String(string) => {
                let string = string.borrow_ref().map_err(ser::Error::custom)?;
                serializer.serialize_str(&string)
            }
            Value::Bytes(bytes) => {
                let bytes = bytes.borrow_ref().map_err(ser::Error::custom)?;
                serializer.serialize_bytes(&bytes)
            }
            Value::Vec(vec) => {
                let vec = vec.borrow_ref().map_err(ser::Error::custom)?;
                let mut serializer = serializer.serialize_seq(Some(vec.len()))?;

                for value in &*vec {
                    serializer.serialize_element(value)?;
                }

                serializer.end()
            }
            Value::EmptyTuple => serializer.serialize_unit(),
            Value::Tuple(tuple) => {
                let tuple = tuple.borrow_ref().map_err(ser::Error::custom)?;
                let mut serializer = serializer.serialize_seq(Some(tuple.len()))?;

                for value in tuple.iter() {
                    serializer.serialize_element(value)?;
                }

                serializer.end()
            }
            Value::Object(object) => {
                let object = object.borrow_ref().map_err(ser::Error::custom)?;
                let mut serializer = serializer.serialize_map(Some(object.len()))?;

                for (key, value) in &*object {
                    serializer.serialize_entry(key, value)?;
                }

                serializer.end()
            }
            Value::Option(option) => {
                let option = option.borrow_ref().map_err(ser::Error::custom)?;
                <Option<Value>>::serialize(&*option, serializer)
            }
            Value::EmptyStruct(..) => serializer.serialize_unit(),
            Value::TupleStruct(..) => Err(ser::Error::custom("cannot serialize tuple structs")),
            Value::Struct(..) => Err(ser::Error::custom("cannot serialize objects structs")),
            Value::Variant(..) => Err(ser::Error::custom("cannot serialize variants")),
            Value::Result(..) => Err(ser::Error::custom("cannot serialize results")),
            Value::Future(..) => Err(ser::Error::custom("cannot serialize futures")),
            Value::Stream(..) => Err(ser::Error::custom("cannot serialize streams")),
            Value::Generator(..) => Err(ser::Error::custom("cannot serialize generators")),
            Value::GeneratorState(..) => {
                Err(ser::Error::custom("cannot serialize generator states"))
            }
            Value::Function(..) => Err(ser::Error::custom("cannot serialize function pointers")),
            Value::Format(..) => Err(ser::Error::custom("cannot serialize format specifications")),
            Value::Iterator(..) => Err(ser::Error::custom("cannot serialize iterators")),
            Value::RangeFrom(..) => Err(ser::Error::custom("cannot serialize `start..` ranges")),
            Value::RangeFull(..) => Err(ser::Error::custom("cannot serialize `..` ranges")),
            Value::RangeInclusive(..) => {
                Err(ser::Error::custom("cannot serialize `start..=end` ranges"))
            }
            Value::RangeToInclusive(..) => {
                Err(ser::Error::custom("cannot serialize `..=end` ranges"))
            }
            Value::RangeTo(..) => Err(ser::Error::custom("cannot serialize `..end` ranges")),
            Value::Range(..) => Err(ser::Error::custom("cannot serialize `start..end` ranges")),
            Value::ControlFlow(..) => {
                Err(ser::Error::custom("cannot serialize `start..end` ranges"))
            }
            Value::Any(..) => Err(ser::Error::custom("cannot serialize external objects")),
        }
    }
}

struct VmVisitor;

impl<'de> de::Visitor<'de> for VmVisitor {
    type Value = Value;

    #[inline]
    fn expecting(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.write_str("any valid value")
    }

    #[inline]
    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let value = value.try_to_owned().map_err(E::custom)?;
        Ok(Value::String(Shared::new(value).map_err(E::custom)?))
    }

    #[inline]
    fn visit_string<E>(self, value: ::rust_alloc::string::String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let value = alloc::String::try_from(value).map_err(E::custom)?;
        Ok(Value::String(Shared::new(value).map_err(E::custom)?))
    }

    #[inline]
    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let v = alloc::Vec::try_from(v).map_err(E::custom)?;
        Ok(Value::Bytes(
            Shared::new(Bytes::from_vec(v)).map_err(E::custom)?,
        ))
    }

    #[inline]
    fn visit_byte_buf<E>(self, v: ::rust_alloc::vec::Vec<u8>) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let v = alloc::Vec::try_from(v).map_err(E::custom)?;
        Ok(Value::Bytes(
            Shared::new(Bytes::from_vec(v)).map_err(E::custom)?,
        ))
    }

    #[inline]
    fn visit_i8<E>(self, v: i8) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::Integer(v as i64))
    }

    #[inline]
    fn visit_i16<E>(self, v: i16) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::Integer(v as i64))
    }

    #[inline]
    fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::Integer(v as i64))
    }

    #[inline]
    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::Integer(v))
    }

    #[inline]
    fn visit_i128<E>(self, v: i128) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::Integer(v as i64))
    }

    #[inline]
    fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::Integer(v as i64))
    }

    #[inline]
    fn visit_u16<E>(self, v: u16) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::Integer(v as i64))
    }

    #[inline]
    fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::Integer(v as i64))
    }

    #[inline]
    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::Integer(v as i64))
    }

    #[inline]
    fn visit_u128<E>(self, v: u128) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::Integer(v as i64))
    }

    #[inline]
    fn visit_f32<E>(self, v: f32) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::Float(v as f64))
    }

    #[inline]
    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::Float(v))
    }

    #[inline]
    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::Bool(v))
    }

    #[inline]
    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let option =
            Shared::new(Some(Value::deserialize(deserializer)?)).map_err(D::Error::custom)?;
        Ok(Value::Option(option))
    }

    #[inline]
    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::Option(Shared::new(None).map_err(E::custom)?))
    }

    #[inline]
    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::EmptyTuple)
    }

    #[inline]
    fn visit_seq<V>(self, mut visitor: V) -> Result<Self::Value, V::Error>
    where
        V: de::SeqAccess<'de>,
    {
        let mut vec = if let Some(hint) = visitor.size_hint() {
            alloc::Vec::try_with_capacity(hint).map_err(V::Error::custom)?
        } else {
            alloc::Vec::new()
        };

        while let Some(elem) = visitor.next_element()? {
            vec.try_push(elem).map_err(V::Error::custom)?;
        }

        Ok(Value::Vec(
            Shared::new(Vec::from(vec)).map_err(V::Error::custom)?,
        ))
    }

    #[inline]
    fn visit_map<V>(self, mut visitor: V) -> Result<Self::Value, V::Error>
    where
        V: de::MapAccess<'de>,
    {
        let mut object = Object::new();

        while let Some((key, value)) = visitor.next_entry()? {
            object.insert(key, value).map_err(V::Error::custom)?;
        }

        Ok(Value::Object(
            Shared::new(object).map_err(V::Error::custom)?,
        ))
    }
}
