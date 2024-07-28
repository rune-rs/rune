use core::fmt;

use crate::alloc;
use crate::alloc::prelude::*;
use crate::runtime::{Bytes, Inline, Mutable, Object, ValueBorrowRef, Vec};

use serde::de::{self, Deserialize as _, Error as _};
use serde::ser::{self, Error as _, SerializeMap as _, SerializeSeq as _};

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
        match self.borrow_ref().map_err(S::Error::custom)? {
            ValueBorrowRef::Inline(value) => match value {
                Inline::Unit => serializer.serialize_unit(),
                Inline::Bool(b) => serializer.serialize_bool(*b),
                Inline::Char(c) => serializer.serialize_char(*c),
                Inline::Byte(c) => serializer.serialize_u8(*c),
                Inline::Integer(integer) => serializer.serialize_i64(*integer),
                Inline::Float(float) => serializer.serialize_f64(*float),
                Inline::Type(..) => Err(ser::Error::custom("cannot serialize types")),
                Inline::Ordering(..) => Err(ser::Error::custom("cannot serialize orderings")),
            },
            ValueBorrowRef::Mutable(value) => match &*value {
                Mutable::String(string) => serializer.serialize_str(string),
                Mutable::Bytes(bytes) => serializer.serialize_bytes(bytes),
                Mutable::Vec(vec) => {
                    let mut serializer = serializer.serialize_seq(Some(vec.len()))?;

                    for value in vec {
                        serializer.serialize_element(value)?;
                    }

                    serializer.end()
                }
                Mutable::Tuple(tuple) => {
                    let mut serializer = serializer.serialize_seq(Some(tuple.len()))?;

                    for value in tuple.iter() {
                        serializer.serialize_element(value)?;
                    }

                    serializer.end()
                }
                Mutable::Object(object) => {
                    let mut serializer = serializer.serialize_map(Some(object.len()))?;

                    for (key, value) in object {
                        serializer.serialize_entry(key, value)?;
                    }

                    serializer.end()
                }
                Mutable::Option(option) => <Option<Value>>::serialize(option, serializer),
                Mutable::EmptyStruct(..) => {
                    Err(ser::Error::custom("cannot serialize empty structs"))
                }
                Mutable::TupleStruct(..) => {
                    Err(ser::Error::custom("cannot serialize tuple structs"))
                }
                Mutable::Struct(..) => Err(ser::Error::custom("cannot serialize objects structs")),
                Mutable::Variant(..) => Err(ser::Error::custom("cannot serialize variants")),
                Mutable::Result(..) => Err(ser::Error::custom("cannot serialize results")),
                Mutable::Future(..) => Err(ser::Error::custom("cannot serialize futures")),
                Mutable::Stream(..) => Err(ser::Error::custom("cannot serialize streams")),
                Mutable::Generator(..) => Err(ser::Error::custom("cannot serialize generators")),
                Mutable::GeneratorState(..) => {
                    Err(ser::Error::custom("cannot serialize generator states"))
                }
                Mutable::Function(..) => {
                    Err(ser::Error::custom("cannot serialize function pointers"))
                }
                Mutable::Format(..) => {
                    Err(ser::Error::custom("cannot serialize format specifications"))
                }
                Mutable::RangeFrom(..) => {
                    Err(ser::Error::custom("cannot serialize `start..` ranges"))
                }
                Mutable::RangeFull(..) => Err(ser::Error::custom("cannot serialize `..` ranges")),
                Mutable::RangeInclusive(..) => {
                    Err(ser::Error::custom("cannot serialize `start..=end` ranges"))
                }
                Mutable::RangeToInclusive(..) => {
                    Err(ser::Error::custom("cannot serialize `..=end` ranges"))
                }
                Mutable::RangeTo(..) => Err(ser::Error::custom("cannot serialize `..end` ranges")),
                Mutable::Range(..) => {
                    Err(ser::Error::custom("cannot serialize `start..end` ranges"))
                }
                Mutable::ControlFlow(..) => {
                    Err(ser::Error::custom("cannot serialize `start..end` ranges"))
                }
                Mutable::Any(..) => Err(ser::Error::custom("cannot serialize external objects")),
            },
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
    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let v = v.try_to_owned().map_err(E::custom)?;
        Value::try_from(v).map_err(E::custom)
    }

    #[inline]
    fn visit_string<E>(self, v: ::rust_alloc::string::String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let v = alloc::String::try_from(v).map_err(E::custom)?;
        Value::try_from(v).map_err(E::custom)
    }

    #[inline]
    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let v = alloc::Vec::try_from(v).map_err(E::custom)?;
        let v = Bytes::from_vec(v);
        Value::try_from(v).map_err(E::custom)
    }

    #[inline]
    fn visit_byte_buf<E>(self, v: ::rust_alloc::vec::Vec<u8>) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let v = alloc::Vec::try_from(v).map_err(E::custom)?;
        let v = Bytes::from_vec(v);
        Value::try_from(v).map_err(E::custom)
    }

    #[inline]
    fn visit_i8<E>(self, v: i8) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::from(v as i64))
    }

    #[inline]
    fn visit_i16<E>(self, v: i16) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::from(v as i64))
    }

    #[inline]
    fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::from(v as i64))
    }

    #[inline]
    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::from(v))
    }

    #[inline]
    fn visit_i128<E>(self, v: i128) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::from(v as i64))
    }

    #[inline]
    fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::from(v as i64))
    }

    #[inline]
    fn visit_u16<E>(self, v: u16) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::from(v as i64))
    }

    #[inline]
    fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::from(v as i64))
    }

    #[inline]
    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::from(v as i64))
    }

    #[inline]
    fn visit_u128<E>(self, v: u128) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::from(v as i64))
    }

    #[inline]
    fn visit_f32<E>(self, v: f32) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::from(v as f64))
    }

    #[inline]
    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::from(v))
    }

    #[inline]
    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::from(v))
    }

    #[inline]
    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let some = Some(Value::deserialize(deserializer)?);
        Value::try_from(some).map_err(D::Error::custom)
    }

    #[inline]
    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Value::try_from(None).map_err(E::custom)
    }

    #[inline]
    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::unit())
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

        let vec = Vec::from(vec);
        Value::try_from(vec).map_err(V::Error::custom)
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

        Value::try_from(object).map_err(V::Error::custom)
    }
}
