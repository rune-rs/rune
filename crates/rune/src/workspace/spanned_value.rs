//! Definition of a TOML spanned value
//!
//! Copied from <https://github.com/est31/toml-spanned-value/tree/e925c52d22dc92c7147cf13ddbd43f78e02b8c32>.

#![allow(unused)]

use core::fmt;
use core::mem::discriminant;
use core::ops;
use core::str::FromStr;

use rust_alloc::string::{String, ToString};
use rust_alloc::vec::Vec;

use crate as rune;
use crate::alloc::prelude::*;

use serde::de;
use serde::ser;

pub(crate) use toml::value::{Datetime, DatetimeParseError};
use toml::Spanned;

pub(crate) use linked_hash_map::LinkedHashMap as Map;

/// Type representing a value with a span
pub(crate) type SpannedValue = Spanned<Value>;

/// Representation of a TOML value.
#[derive(PartialEq, Clone, Debug)]
pub(crate) enum Value {
    /// Represents a TOML string
    String(String),
    /// Represents a TOML integer
    Integer(i64),
    /// Represents a TOML float
    Float(f64),
    /// Represents a TOML boolean
    Boolean(bool),
    /// Represents a TOML datetime
    Datetime(Datetime),
    /// Represents a TOML array
    Array(Array),
    /// Represents a TOML table
    Table(Table),
}

/// Type representing a TOML array, payload of the `Value::Array` variant
pub(crate) type Array = Vec<SpannedValue>;

/// Type representing a TOML table, payload of the `Value::Table` variant.
/// By default it is backed by a BTreeMap, enable the `preserve_order` feature
/// to use a LinkedHashMap instead.
pub(crate) type Table = Map<Spanned<String>, SpannedValue>;

impl Value {
    /* /// Interpret a `toml::Value` as an instance of type `T`.
    ///
    /// This conversion can fail if the structure of the `Value` does not match the
    /// structure expected by `T`, for example if `T` is a struct type but the
    /// `Value` contains something other than a TOML table. It can also fail if the
    /// structure is correct but `T`'s implementation of `Deserialize` decides that
    /// something is wrong with the data, for example required struct fields are
    /// missing from the TOML map or some number is too big to fit in the expected
    /// primitive type.
    pub(crate) fn try_into<'de, T>(self) -> Result<T, crate::de::Error>
    where
        T: de::Deserialize<'de>,
    {
        de::Deserialize::deserialize(self)
    }*/

    /// Index into a TOML array or map. A string index can be used to access a
    /// value in a map, and a usize index can be used to access an element of an
    /// array.
    ///
    /// Returns `None` if the type of `self` does not match the type of the
    /// index, for example if the index is a string and `self` is an array or a
    /// number. Also returns `None` if the given key does not exist in the map
    /// or the given index is not within the bounds of the array.
    pub(crate) fn get<I: Index>(&self, index: I) -> Option<&SpannedValue> {
        index.index(self)
    }

    /// Mutably index into a TOML array or map. A string index can be used to
    /// access a value in a map, and a usize index can be used to access an
    /// element of an array.
    ///
    /// Returns `None` if the type of `self` does not match the type of the
    /// index, for example if the index is a string and `self` is an array or a
    /// number. Also returns `None` if the given key does not exist in the map
    /// or the given index is not within the bounds of the array.
    pub(crate) fn get_mut<I: Index>(&mut self, index: I) -> Option<&mut SpannedValue> {
        index.index_mut(self)
    }

    /// Extracts the integer value if it is an integer.
    pub(crate) fn as_integer(&self) -> Option<i64> {
        match *self {
            Value::Integer(i) => Some(i),
            _ => None,
        }
    }

    /// Tests whether this value is an integer.
    pub(crate) fn is_integer(&self) -> bool {
        self.as_integer().is_some()
    }

    /// Extracts the float value if it is a float.
    pub(crate) fn as_float(&self) -> Option<f64> {
        match *self {
            Value::Float(f) => Some(f),
            _ => None,
        }
    }

    /// Tests whether this value is a float.
    pub(crate) fn is_float(&self) -> bool {
        self.as_float().is_some()
    }

    /// Extracts the boolean value if it is a boolean.
    pub(crate) fn as_bool(&self) -> Option<bool> {
        match *self {
            Value::Boolean(b) => Some(b),
            _ => None,
        }
    }

    /// Tests whether this value is a boolean.
    pub(crate) fn is_bool(&self) -> bool {
        self.as_bool().is_some()
    }

    /// Extracts the string of this value if it is a string.
    pub(crate) fn as_str(&self) -> Option<&str> {
        match *self {
            Value::String(ref s) => Some(&**s),
            _ => None,
        }
    }

    /// Tests if this value is a string.
    pub(crate) fn is_str(&self) -> bool {
        self.as_str().is_some()
    }

    /// Extracts the datetime value if it is a datetime.
    ///
    /// Note that a parsed TOML value will only contain ISO 8601 dates. An
    /// example date is:
    ///
    /// ```notrust
    /// 1979-05-27T07:32:00Z
    /// ```
    pub(crate) fn as_datetime(&self) -> Option<&Datetime> {
        match *self {
            Value::Datetime(ref s) => Some(s),
            _ => None,
        }
    }

    /// Tests whether this value is a datetime.
    pub(crate) fn is_datetime(&self) -> bool {
        self.as_datetime().is_some()
    }

    /// Extracts the array value if it is an array.
    pub(crate) fn as_array(&self) -> Option<&Vec<SpannedValue>> {
        match *self {
            Value::Array(ref s) => Some(s),
            _ => None,
        }
    }

    /// Extracts the array value if it is an array.
    pub(crate) fn as_array_mut(&mut self) -> Option<&mut Vec<SpannedValue>> {
        match *self {
            Value::Array(ref mut s) => Some(s),
            _ => None,
        }
    }

    /// Tests whether this value is an array.
    pub(crate) fn is_array(&self) -> bool {
        self.as_array().is_some()
    }

    /// Extracts the table value if it is a table.
    pub(crate) fn as_table(&self) -> Option<&Table> {
        match *self {
            Value::Table(ref s) => Some(s),
            _ => None,
        }
    }

    /// Extracts the table value if it is a table.
    pub(crate) fn as_table_mut(&mut self) -> Option<&mut Table> {
        match *self {
            Value::Table(ref mut s) => Some(s),
            _ => None,
        }
    }

    /// Tests whether this value is a table.
    pub(crate) fn is_table(&self) -> bool {
        self.as_table().is_some()
    }

    /// Tests whether this and another value have the same type.
    pub(crate) fn same_type(&self, other: &Value) -> bool {
        discriminant(self) == discriminant(other)
    }

    /// Returns a human-readable representation of the type of this value.
    pub(crate) fn type_str(&self) -> &'static str {
        match *self {
            Value::String(..) => "string",
            Value::Integer(..) => "integer",
            Value::Float(..) => "float",
            Value::Boolean(..) => "boolean",
            Value::Datetime(..) => "datetime",
            Value::Array(..) => "array",
            Value::Table(..) => "table",
        }
    }
}

impl<I> ops::Index<I> for Value
where
    I: Index,
{
    type Output = SpannedValue;

    fn index(&self, index: I) -> &SpannedValue {
        self.get(index).expect("index not found")
    }
}

impl<I> ops::IndexMut<I> for Value
where
    I: Index,
{
    fn index_mut(&mut self, index: I) -> &mut SpannedValue {
        self.get_mut(index).expect("index not found")
    }
}

impl From<&str> for Value {
    #[inline]
    fn from(string: &str) -> Value {
        Value::String(string.to_string())
    }
}

impl<V> From<Vec<V>> for Value
where
    V: Into<SpannedValue>,
{
    fn from(val: Vec<V>) -> Value {
        Value::Array(val.into_iter().map(|v| v.into()).collect())
    }
}

macro_rules! impl_into_value {
    ($variant:ident : $T:ty) => {
        impl From<$T> for Value {
            #[inline]
            fn from(val: $T) -> Value {
                Value::$variant(val.into())
            }
        }
    };
}

impl_into_value!(String: String);
impl_into_value!(Integer: i64);
impl_into_value!(Integer: i32);
impl_into_value!(Integer: i8);
impl_into_value!(Integer: u8);
impl_into_value!(Integer: u32);
impl_into_value!(Float: f64);
impl_into_value!(Float: f32);
impl_into_value!(Boolean: bool);
impl_into_value!(Datetime: Datetime);
impl_into_value!(Table: Table);

/// Types that can be used to index a `toml::Value`
///
/// Currently this is implemented for `usize` to index arrays and `str` to index
/// tables.
///
/// This trait is sealed and not intended for implementation outside of the
/// `toml` crate.
pub(crate) trait Index: Sealed {
    #[doc(hidden)]
    fn index<'a>(&self, val: &'a Value) -> Option<&'a SpannedValue>;
    #[doc(hidden)]
    fn index_mut<'a>(&self, val: &'a mut Value) -> Option<&'a mut SpannedValue>;
}

/// An implementation detail that should not be implemented, this will change in
/// the future and break code otherwise.
#[doc(hidden)]
pub(crate) trait Sealed {}
impl Sealed for usize {}
impl Sealed for str {}
impl Sealed for String {}
impl<'a, T: Sealed + ?Sized> Sealed for &'a T {}

impl Index for usize {
    fn index<'a>(&self, val: &'a Value) -> Option<&'a SpannedValue> {
        match *val {
            Value::Array(ref a) => a.get(*self),
            _ => None,
        }
    }

    fn index_mut<'a>(&self, val: &'a mut Value) -> Option<&'a mut SpannedValue> {
        match *val {
            Value::Array(ref mut a) => a.get_mut(*self),
            _ => None,
        }
    }
}

impl Index for str {
    fn index<'a>(&self, val: &'a Value) -> Option<&'a SpannedValue> {
        match *val {
            Value::Table(ref a) => a.get(self),
            _ => None,
        }
    }

    fn index_mut<'a>(&self, val: &'a mut Value) -> Option<&'a mut SpannedValue> {
        match *val {
            Value::Table(ref mut a) => a.get_mut(self),
            _ => None,
        }
    }
}

impl Index for String {
    fn index<'a>(&self, val: &'a Value) -> Option<&'a SpannedValue> {
        self[..].index(val)
    }

    fn index_mut<'a>(&self, val: &'a mut Value) -> Option<&'a mut SpannedValue> {
        self[..].index_mut(val)
    }
}

impl<'s, T: ?Sized> Index for &'s T
where
    T: Index,
{
    fn index<'a>(&self, val: &'a Value) -> Option<&'a SpannedValue> {
        (**self).index(val)
    }

    fn index_mut<'a>(&self, val: &'a mut Value) -> Option<&'a mut SpannedValue> {
        (**self).index_mut(val)
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        toml::ser::to_string(self)
            .expect("Unable to represent value as string")
            .fmt(f)
    }
}

impl FromStr for Value {
    type Err = toml::de::Error;
    fn from_str(s: &str) -> Result<Value, Self::Err> {
        toml::from_str(s)
    }
}

impl ser::Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        use serde::ser::SerializeMap;

        match *self {
            Value::String(ref s) => serializer.serialize_str(s),
            Value::Integer(i) => serializer.serialize_i64(i),
            Value::Float(f) => serializer.serialize_f64(f),
            Value::Boolean(b) => serializer.serialize_bool(b),
            Value::Datetime(ref s) => s.serialize(serializer),
            Value::Array(ref a) => a.serialize(serializer),
            Value::Table(ref t) => {
                let mut map = serializer.serialize_map(Some(t.len()))?;
                // Be sure to visit non-tables first (and also non
                // array-of-tables) as all keys must be emitted first.
                for (k, v) in t {
                    if !v.get_ref().is_table() && !v.get_ref().is_array()
                        || (v
                            .get_ref()
                            .as_array()
                            .map(|a| !a.iter().any(|v| v.get_ref().is_table()))
                            .unwrap_or(false))
                    {
                        map.serialize_entry(k, v)?;
                    }
                }
                for (k, v) in t {
                    if v.get_ref()
                        .as_array()
                        .map(|a| a.iter().any(|v| v.get_ref().is_table()))
                        .unwrap_or(false)
                    {
                        map.serialize_entry(k, v)?;
                    }
                }
                for (k, v) in t {
                    if v.get_ref().is_table() {
                        map.serialize_entry(k, v)?;
                    }
                }
                map.end()
            }
        }
    }
}

impl<'de> de::Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Value, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct ValueKindVisitor;

        impl<'de> de::Visitor<'de> for ValueKindVisitor {
            type Value = Value;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("any valid TOML value")
            }

            fn visit_bool<E>(self, value: bool) -> Result<Value, E> {
                Ok(Value::Boolean(value))
            }

            fn visit_i64<E>(self, value: i64) -> Result<Value, E> {
                Ok(Value::Integer(value))
            }

            fn visit_u64<E>(self, value: u64) -> Result<Value, E>
            where
                E: de::Error,
            {
                if value <= i64::max_value() as u64 {
                    Ok(Value::Integer(value as i64))
                } else {
                    Err(de::Error::custom("u64 value was too large"))
                }
            }

            fn visit_u32<E>(self, value: u32) -> Result<Value, E> {
                Ok(Value::Integer(value.into()))
            }

            fn visit_i32<E>(self, value: i32) -> Result<Value, E> {
                Ok(Value::Integer(value.into()))
            }

            fn visit_f64<E>(self, value: f64) -> Result<Value, E> {
                Ok(Value::Float(value))
            }

            fn visit_str<E>(self, value: &str) -> Result<Value, E>
            where
                E: de::Error,
            {
                Ok(Value::String(value.to_string()))
            }

            fn visit_string<E>(self, value: String) -> Result<Value, E>
            where
                E: de::Error,
            {
                Ok(Value::String(value))
            }

            fn visit_some<D>(self, deserializer: D) -> Result<Value, D::Error>
            where
                D: de::Deserializer<'de>,
            {
                de::Deserialize::deserialize(deserializer)
            }

            fn visit_seq<V>(self, mut visitor: V) -> Result<Value, V::Error>
            where
                V: de::SeqAccess<'de>,
            {
                let mut vec = Vec::new();
                while let Some(elem) = visitor.next_element()? {
                    vec.push(elem);
                }
                Ok(Value::Array(vec))
            }

            fn visit_map<V>(self, mut visitor: V) -> Result<Value, V::Error>
            where
                V: de::MapAccess<'de>,
            {
                let key = visitor.next_key_seed(DatetimeOrTable)?;
                let key = match key {
                    Some(Some(key)) => key,
                    Some(None) => {
                        let date: DatetimeFromString = visitor.next_value()?;
                        return Ok(Value::Datetime(date.value));
                    }
                    None => return Ok(Value::Table(Map::new())),
                };
                let mut map = Map::new();
                map.insert(key, visitor.next_value()?);
                while let Some(key) = visitor.next_key()? {
                    if map.contains_key(&key) {
                        let key: Spanned<String> = key;
                        let msg = format!("duplicate key: `{}`", key.get_ref());
                        return Err(de::Error::custom(msg));
                    }
                    map.insert(key, visitor.next_value()?);
                }
                Ok(Value::Table(map))
            }
        }

        deserializer.deserialize_any(ValueKindVisitor)
    }
}

struct DatetimeFromString {
    pub(crate) value: Datetime,
}

impl<'de> de::Deserialize<'de> for DatetimeFromString {
    fn deserialize<D>(deserializer: D) -> Result<DatetimeFromString, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = DatetimeFromString;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("string containing a datetime")
            }

            fn visit_str<E>(self, s: &str) -> Result<DatetimeFromString, E>
            where
                E: de::Error,
            {
                match s.parse() {
                    Ok(date) => Ok(DatetimeFromString { value: date }),
                    Err(e) => Err(de::Error::custom(e)),
                }
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}

#[derive(Debug)]
struct OptError<E: de::Error>(Option<E>);

impl<E: de::Error> std::error::Error for OptError<E> {}

impl<E: de::Error> core::fmt::Display for OptError<E> {
    fn fmt(&self, _fmt: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        // The error is never meant to be displayed.
        // Our code is expected to unwrap the error before
        // it is propagated to places that may display it.
        unreachable!()
    }
}

impl<E: de::Error> de::Error for OptError<E> {
    fn custom<T: core::fmt::Display>(msg: T) -> Self {
        Self(Some(<E as de::Error>::custom(msg)))
    }
}

struct LayerDeserializer<'de, D: de::Deserializer<'de>>(D, std::marker::PhantomData<&'de ()>);

impl<'de, D: de::Deserializer<'de>> de::Deserializer<'de> for LayerDeserializer<'de, D> {
    type Error = OptError<D::Error>;
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.0
            .deserialize_any(visitor)
            .map_err(|e| OptError(Some(e)))
    }
    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let wrapped_visitor = DatetimeOrTableWrapper(visitor);
        match self.0.deserialize_struct(name, fields, wrapped_visitor) {
            Ok(Some(v)) => Ok(v),
            Ok(None) => Err(OptError(None)),
            Err(v) => Err(OptError(Some(v))),
        }
    }
    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map enum identifier ignored_any
    }
}

struct DatetimeOrTable;

impl<'de> de::DeserializeSeed<'de> for DatetimeOrTable {
    type Value = Option<Spanned<String>>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let deserializer = LayerDeserializer(deserializer, std::marker::PhantomData);
        let res = <Spanned<String> as de::Deserialize<'_>>::deserialize(deserializer);
        match res {
            Ok(v) => Ok(Some(v)),
            Err(OptError(None)) => Ok(None),
            Err(OptError(Some(e))) => Err(e),
        }
    }
}

struct DatetimeOrTableWrapper<V>(V);

impl<'de, V: de::Visitor<'de>> de::Visitor<'de> for DatetimeOrTableWrapper<V> {
    type Value = Option<V::Value>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a string key")
    }

    fn visit_map<W>(self, visitor: W) -> Result<Self::Value, W::Error>
    where
        W: de::MapAccess<'de>,
    {
        let key = self.0.visit_map(visitor)?;
        Ok(Some(key))
    }

    fn visit_str<E>(self, _s: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        //assert_eq!(s, toml::datetime::FIELD);
        Ok(None)
    }

    fn visit_string<E>(self, _s: rust_alloc::string::String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        //assert_eq!(s, toml::datetime::FIELD);
        Ok(None)
    }
}
