use crate::bytes::Bytes;
use crate::collections::HashMap;
use crate::error;
use crate::tls;
use crate::value::Value;
use crate::vm::VmError;
use serde::{de, ser};
use std::fmt;

/// Deserialize implementation for value pointers.
///
/// **Warning:** This only works if a `Vm` is accessible through [tls], like by
/// being set up with [tls::inject_vm] or [tls::InjectVm].
impl<'de> de::Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_any(VmVisitor)
    }
}

/// Serialize implementation for value pointers.
///
/// **Warning:** This only works if a `Vm` is accessible through [tls], like by
/// being set up with [tls::inject_vm] or [tls::InjectVm].
impl ser::Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        use serde::ser::SerializeMap as _;
        use serde::ser::SerializeSeq as _;

        match *self {
            Value::Unit => serializer.serialize_unit(),
            Value::Bool(b) => serializer.serialize_bool(b),
            Value::Char(c) => serializer.serialize_char(c),
            Value::Byte(c) => serializer.serialize_u8(c),
            Value::Integer(integer) => serializer.serialize_i64(integer),
            Value::Float(float) => serializer.serialize_f64(float),
            Value::StaticString(slot) => tls::with_vm(|vm| {
                let string = vm.unit.lookup_string(slot).map_err(ser::Error::custom)?;
                serializer.serialize_str(string)
            }),
            Value::String(slot) => tls::with_vm(|vm| {
                let string = vm.string_ref(slot).map_err(ser::Error::custom)?;
                serializer.serialize_str(&*string)
            }),
            Value::Bytes(slot) => tls::with_vm(|vm| {
                let bytes = vm.bytes_ref(slot).map_err(ser::Error::custom)?;
                serializer.serialize_bytes(&*bytes)
            }),
            Value::Vec(slot) => tls::with_vm(|vm| {
                let vec = vm.vec_ref(slot).map_err(ser::Error::custom)?;
                let mut serializer = serializer.serialize_seq(Some(vec.len()))?;

                for value in &*vec {
                    serializer.serialize_element(value)?;
                }

                serializer.end()
            }),
            Value::Tuple(slot) => tls::with_vm(|vm| {
                let tuple = vm
                    .external_ref::<Box<[Value]>>(slot)
                    .map_err(ser::Error::custom)?;
                let mut serializer = serializer.serialize_seq(Some(tuple.len()))?;

                for value in tuple.iter() {
                    serializer.serialize_element(value)?;
                }

                serializer.end()
            }),
            Value::Object(slot) => tls::with_vm(|vm| {
                let object = vm.object_ref(slot).map_err(ser::Error::custom)?;
                let mut serializer = serializer.serialize_map(Some(object.len()))?;

                for (key, value) in &*object {
                    serializer.serialize_entry(key, value)?;
                }

                serializer.end()
            }),
            Value::Option(slot) => tls::with_vm(|vm| {
                let option = vm.option_ref(slot).map_err(ser::Error::custom)?;
                <Option<Value>>::serialize(&*option, serializer)
            }),
            Value::TypedTuple(..) => Err(ser::Error::custom("cannot serialize tuple types")),
            Value::Result(..) => Err(ser::Error::custom("cannot serialize results")),
            Value::Type(..) => Err(ser::Error::custom("cannot serialize types")),
            Value::Future(..) => Err(ser::Error::custom("cannot serialize futures")),
            Value::External(..) => Err(ser::Error::custom("cannot serialize external objects")),
        }
    }
}

impl de::Error for VmError {
    fn custom<T>(msg: T) -> Self
    where
        T: fmt::Display,
    {
        VmError::UserError {
            error: error::Error::msg(msg.to_string()),
        }
    }
}

struct VmVisitor;

impl<'de> de::Visitor<'de> for VmVisitor {
    type Value = Value;

    fn expecting(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.write_str("any valid value")
    }

    #[inline]
    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        tls::with_vm(|vm| Ok(vm.string_allocate(value.to_owned())))
    }

    #[inline]
    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        tls::with_vm(|vm| Ok(vm.string_allocate(value)))
    }

    #[inline]
    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        tls::with_vm(|vm| Ok(vm.external_allocate(Bytes::from_vec(v.to_vec()))))
    }

    #[inline]
    fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        tls::with_vm(|vm| Ok(vm.external_allocate(Bytes::from_vec(v))))
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
    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::Bool(v))
    }

    #[inline]
    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::Unit)
    }

    #[inline]
    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::Unit)
    }

    #[inline]
    fn visit_seq<V>(self, mut visitor: V) -> Result<Self::Value, V::Error>
    where
        V: de::SeqAccess<'de>,
    {
        let mut vec = Vec::new();

        while let Some(elem) = visitor.next_element()? {
            vec.push(elem);
        }

        tls::with_vm(|vm| Ok(vm.vec_allocate(vec)))
    }

    #[inline]
    fn visit_map<V>(self, mut visitor: V) -> Result<Self::Value, V::Error>
    where
        V: de::MapAccess<'de>,
    {
        let mut object = HashMap::<String, Value>::new();

        while let Some((key, value)) = visitor.next_entry()? {
            object.insert(key, value);
        }

        tls::with_vm(|vm| Ok(vm.object_allocate(object)))
    }
}
