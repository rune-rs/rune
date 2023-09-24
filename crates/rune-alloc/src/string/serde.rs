use core::fmt;

use serde::de::{self, Error, Unexpected};
use serde::ser;

use crate::borrow::{Cow, TryToOwned};
use crate::{Box, String, Vec};

impl ser::Serialize for String {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> de::Deserialize<'de> for String {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_string(StringVisitor)
    }

    fn deserialize_in_place<D>(deserializer: D, place: &mut Self) -> Result<(), D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_string(StringInPlaceVisitor(place))
    }
}

impl<'de> de::Deserialize<'de> for Box<str> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let string = String::deserialize(deserializer)?;
        string.try_into_boxed_str().map_err(D::Error::custom)
    }
}

impl<'de, 'a, T: ?Sized> de::Deserialize<'de> for Cow<'a, T>
where
    T: TryToOwned,
    T::Owned: de::Deserialize<'de>,
{
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        T::Owned::deserialize(deserializer).map(Cow::Owned)
    }
}

struct StringVisitor;
struct StringInPlaceVisitor<'a>(&'a mut String);

impl<'de> de::Visitor<'de> for StringVisitor {
    type Value = String;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        v.try_to_owned().map_err(E::custom)
    }

    fn visit_string<E>(self, v: ::rust_alloc::string::String) -> Result<Self::Value, E>
    where
        E: Error,
    {
        String::try_from(v).map_err(E::custom)
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: Error,
    {
        match core::str::from_utf8(v) {
            Ok(s) => s.try_to_owned().map_err(E::custom),
            Err(_) => Err(Error::invalid_value(Unexpected::Bytes(v), &self)),
        }
    }

    fn visit_byte_buf<E>(self, v: ::rust_alloc::vec::Vec<u8>) -> Result<Self::Value, E>
    where
        E: Error,
    {
        let v = Vec::try_from(v).map_err(E::custom)?;

        match String::from_utf8(v) {
            Ok(s) => Ok(s),
            Err(e) => Err(Error::invalid_value(
                Unexpected::Bytes(&e.into_bytes()),
                &self,
            )),
        }
    }
}

impl<'a, 'de> de::Visitor<'de> for StringInPlaceVisitor<'a> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        self.0.clear();
        self.0.try_push_str(v).map_err(E::custom)?;
        Ok(())
    }

    fn visit_string<E>(self, v: ::rust_alloc::string::String) -> Result<Self::Value, E>
    where
        E: Error,
    {
        *self.0 = String::try_from(v.as_str()).map_err(E::custom)?;
        Ok(())
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: Error,
    {
        match core::str::from_utf8(v) {
            Ok(s) => {
                self.0.clear();
                self.0.try_push_str(s).map_err(E::custom)?;
                Ok(())
            }
            Err(_) => Err(Error::invalid_value(Unexpected::Bytes(v), &self)),
        }
    }

    fn visit_byte_buf<E>(self, v: ::rust_alloc::vec::Vec<u8>) -> Result<Self::Value, E>
    where
        E: Error,
    {
        let v = Vec::try_from(v).map_err(E::custom)?;

        match String::from_utf8(v) {
            Ok(s) => {
                *self.0 = s;
                Ok(())
            }
            Err(e) => Err(Error::invalid_value(
                Unexpected::Bytes(&e.into_bytes()),
                &self,
            )),
        }
    }
}
