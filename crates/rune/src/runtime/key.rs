use core::fmt;

use crate::no_std::prelude::*;
use crate::no_std::sync::Arc;
use crate::no_std::vec;

use serde::{de, ser};

use crate::runtime::{
    Bytes, FromValue, FullTypeOf, MaybeTypeOf, Object, OwnedTuple, Shared, ToValue, TypeInfo,
    Value, Variant, VariantData, VariantRtti, Vec, VmErrorKind, VmResult,
};

/// A key that can be used as an anonymous object key.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Key {
    /// A constant unit.
    EmptyTuple,
    /// A byte.
    Byte(u8),
    /// A character.
    Char(char),
    /// A boolean constant value.
    Bool(bool),
    /// An integer constant.
    Integer(i64),
    /// A string constant designated by its slot.
    String(Box<str>),
    /// A byte string.
    Bytes(Box<[u8]>),
    /// A vector of values.
    Vec(vec::Vec<Key>),
    /// An anonymous tuple.
    Tuple(Box<[Key]>),
    /// An option.
    Option(Option<Box<Key>>),
    /// A variant.
    Variant(VariantKey),
}

impl Key {
    /// Convert a value reference into a key.
    pub fn from_value(value: &Value) -> VmResult<Self> {
        return VmResult::Ok(match value {
            Value::EmptyTuple => Self::EmptyTuple,
            Value::Byte(b) => Self::Byte(*b),
            Value::Char(c) => Self::Char(*c),
            Value::Bool(b) => Self::Bool(*b),
            Value::Integer(n) => Self::Integer(*n),
            Value::String(s) => Self::String(vm_try!(s.borrow_ref()).as_str().into()),
            Value::Bytes(b) => Self::Bytes(vm_try!(b.borrow_ref()).as_ref().into()),
            Value::Option(option) => Self::Option(match &*vm_try!(option.borrow_ref()) {
                Some(some) => Some(Box::new(vm_try!(Self::from_value(some)))),
                None => None,
            }),
            Value::Vec(vec) => {
                let vec = vm_try!(vec.borrow_ref());
                let mut key_vec = vec::Vec::with_capacity(vec.len());

                for value in &*vec {
                    key_vec.push(vm_try!(Self::from_value(value)));
                }

                Self::Vec(key_vec)
            }
            Value::Tuple(tuple) => {
                let tuple = vm_try!(tuple.borrow_ref());
                Self::Tuple(vm_try!(tuple_from_value(&tuple)))
            }
            Value::Variant(variant) => {
                let variant = vm_try!(variant.borrow_ref());

                let data = match &variant.data {
                    VariantData::Empty => VariantKeyData::Unit,
                    VariantData::Tuple(tuple) => {
                        VariantKeyData::Tuple(vm_try!(tuple_from_value(tuple)))
                    }
                    VariantData::Struct(object) => {
                        VariantKeyData::Struct(vm_try!(struct_from_value(object)))
                    }
                };

                Key::Variant(VariantKey {
                    rtti: variant.rtti.clone(),
                    data,
                })
            }
            value => {
                return VmResult::err(VmErrorKind::KeyNotSupported {
                    actual: vm_try!(value.type_info()),
                });
            }
        });

        fn tuple_from_value(tuple: &OwnedTuple) -> VmResult<Box<[Key]>> {
            let mut output = vec::Vec::with_capacity(tuple.len());

            for value in tuple.iter() {
                output.push(vm_try!(Key::from_value(value)));
            }

            VmResult::Ok(output.into_boxed_slice())
        }

        fn struct_from_value(object: &Object) -> VmResult<Box<[(Box<str>, Key)]>> {
            let mut output = vec::Vec::with_capacity(object.len());

            for (key, value) in object {
                output.push((key.as_str().into(), vm_try!(Key::from_value(value))));
            }

            VmResult::Ok(output.into_boxed_slice())
        }
    }

    /// Convert into virtual machine value.
    ///
    /// We provide this associated method since a constant value can be
    /// converted into a value infallibly, which is not captured by the trait
    /// otherwise.
    pub fn into_value(self) -> Value {
        return match self {
            Self::EmptyTuple => Value::EmptyTuple,
            Self::Byte(b) => Value::Byte(b),
            Self::Char(c) => Value::Char(c),
            Self::Bool(b) => Value::Bool(b),
            Self::Integer(n) => Value::Integer(n),
            Self::String(s) => Value::String(Shared::new(String::from(s))),
            Self::Bytes(b) => Value::Bytes(Shared::new(Bytes::from(b))),
            Self::Option(option) => {
                Value::Option(Shared::new(option.map(|some| some.into_value())))
            }
            Self::Vec(vec) => {
                let mut v = Vec::with_capacity(vec.len());

                for value in vec {
                    v.push(value.into_value());
                }

                Value::Vec(Shared::new(v))
            }
            Self::Tuple(tuple) => Value::Tuple(Shared::new(tuple_into_value(tuple))),
            Self::Variant(variant) => {
                let data = match variant.data {
                    VariantKeyData::Unit => VariantData::Empty,
                    VariantKeyData::Tuple(tuple) => VariantData::Tuple(tuple_into_value(tuple)),
                    VariantKeyData::Struct(st) => VariantData::Struct(struct_into_value(st)),
                };

                Value::Variant(Shared::new(Variant {
                    rtti: variant.rtti,
                    data,
                }))
            }
        };

        fn tuple_into_value(data: Box<[Key]>) -> OwnedTuple {
            let mut t = vec::Vec::with_capacity(data.len());

            for value in vec::Vec::from(data) {
                t.push(value.into_value());
            }

            OwnedTuple::from(t)
        }

        fn struct_into_value(data: Box<[(Box<str>, Key)]>) -> Object {
            let mut object = Object::with_capacity(data.len());

            for (key, value) in vec::Vec::from(data) {
                object.insert(key.into(), value.into_value());
            }

            object
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
            Self::Byte(..) => TypeInfo::StaticType(crate::runtime::static_type::BYTE_TYPE),
            Self::Char(..) => TypeInfo::StaticType(crate::runtime::static_type::CHAR_TYPE),
            Self::Bool(..) => TypeInfo::StaticType(crate::runtime::static_type::BOOL_TYPE),
            Self::String(..) => TypeInfo::StaticType(crate::runtime::static_type::STRING_TYPE),
            Self::Bytes(..) => TypeInfo::StaticType(crate::runtime::static_type::BYTES_TYPE),
            Self::Integer(..) => TypeInfo::StaticType(crate::runtime::static_type::INTEGER_TYPE),
            Self::Vec(..) => TypeInfo::StaticType(crate::runtime::static_type::VEC_TYPE),
            Self::EmptyTuple => TypeInfo::StaticType(crate::runtime::static_type::TUPLE_TYPE),
            Self::Tuple(..) => TypeInfo::StaticType(crate::runtime::static_type::TUPLE_TYPE),
            Self::Option(..) => TypeInfo::StaticType(crate::runtime::static_type::OPTION_TYPE),
            Self::Variant(variant) => TypeInfo::Variant(variant.rtti.clone()),
        }
    }
}

impl fmt::Debug for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Key::Byte(b) => write!(f, "{:?}", b),
            Key::Char(c) => write!(f, "{:?}", c),
            Key::Bool(b) => write!(f, "{}", b),
            Key::Integer(n) => write!(f, "{}", n),
            Key::String(s) => write!(f, "{:?}", s),
            Key::Bytes(b) => write!(f, "{:?}", b),
            Key::Vec(vec) => write!(f, "{:?}", vec),
            Key::EmptyTuple => write!(f, "()"),
            Key::Tuple(tuple) => {
                write!(f, "(")?;

                let mut it = tuple.iter().peekable();

                while let Some(key) = it.next() {
                    write!(f, "{:?}", key)?;

                    if it.peek().is_some() {
                        write!(f, ", ")?;
                    }
                }

                write!(f, "(")?;
                Ok(())
            }
            Key::Option(opt) => write!(f, "{:?}", opt),
            Key::Variant(variant) => write!(f, "{:?}", variant),
        }
    }
}

impl FromValue for Key {
    #[inline]
    fn from_value(value: Value) -> VmResult<Self> {
        Key::from_value(&value)
    }
}

impl ToValue for Key {
    #[inline]
    fn to_value(self) -> VmResult<Value> {
        VmResult::Ok(Key::into_value(self))
    }
}

impl MaybeTypeOf for Key {
    #[inline]
    fn maybe_type_of() -> Option<FullTypeOf> {
        None
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
            Self::Bool(b) => serializer.serialize_bool(*b),
            Self::Char(c) => serializer.serialize_char(*c),
            Self::Byte(c) => serializer.serialize_u8(*c),
            Self::Integer(integer) => serializer.serialize_i64(*integer),
            Self::String(string) => serializer.serialize_str(string),
            Self::Bytes(bytes) => serializer.serialize_bytes(bytes),
            Self::Vec(vec) => {
                let mut serializer = serializer.serialize_seq(Some(vec.len()))?;

                for value in vec {
                    serializer.serialize_element(value)?;
                }

                serializer.end()
            }
            Self::EmptyTuple => serializer.serialize_unit(),
            Self::Tuple(tuple) => {
                let mut serializer = serializer.serialize_seq(Some(tuple.len()))?;

                for value in tuple.iter() {
                    serializer.serialize_element(value)?;
                }

                serializer.end()
            }
            Self::Option(option) => <Option<Box<Key>>>::serialize(option, serializer),
            Self::Variant(..) => Err(ser::Error::custom("cannot serialize variants")),
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
        Ok(Key::String(value.into()))
    }

    #[inline]
    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Key::String(value.into()))
    }

    #[inline]
    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Key::Bytes(v.into()))
    }

    #[inline]
    fn visit_byte_buf<E>(self, v: vec::Vec<u8>) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Key::Bytes(v.into()))
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
        Ok(Key::EmptyTuple)
    }

    #[inline]
    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Key::EmptyTuple)
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

impl From<Box<str>> for Key {
    #[inline]
    fn from(value: Box<str>) -> Self {
        Self::String(value)
    }
}

impl From<String> for Key {
    #[inline]
    fn from(value: String) -> Self {
        Self::String(value.into())
    }
}

impl From<i64> for Key {
    #[inline]
    fn from(value: i64) -> Self {
        Self::Integer(value)
    }
}

/// A variant that has been serialized to a key.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VariantKey {
    rtti: Arc<VariantRtti>,
    data: VariantKeyData,
}

impl fmt::Debug for VariantKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.rtti.item)?;

        match &self.data {
            VariantKeyData::Unit => (),
            VariantKeyData::Tuple(tuple) => {
                let mut it = tuple.iter();
                let last = it.next_back();

                write!(f, "(")?;

                for v in it {
                    write!(f, "{:?}, ", v)?;
                }

                if let Some(v) = last {
                    write!(f, "{:?}", v)?;
                }

                write!(f, ")")?;
            }
            VariantKeyData::Struct(st) => f
                .debug_map()
                .entries(st.iter().map(|(k, v)| (k, v)))
                .finish()?,
        }

        Ok(())
    }
}

/// Variant data that has been serialized to a key.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum VariantKeyData {
    /// A unit variant with a specific type hash.
    Unit,
    /// A tuple variant with a specific type hash.
    Tuple(Box<[Key]>),
    /// An struct variant with a specific type hash.
    Struct(Box<[(Box<str>, Key)]>),
}
