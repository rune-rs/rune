pub(crate) mod ordering {
    use core::cmp::Ordering;

    use serde::{Deserialize, Deserializer, Serializer};

    pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<Ordering, D::Error>
    where
        D: Deserializer<'de>,
    {
        match i8::deserialize(deserializer)? {
            -1 => Ok(Ordering::Less),
            0 => Ok(Ordering::Equal),
            1 => Ok(Ordering::Greater),
            _ => Err(serde::de::Error::custom("invalid ordering")),
        }
    }

    pub(crate) fn serialize<S>(ordering: &Ordering, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match ordering {
            Ordering::Less => serializer.serialize_i8(-1),
            Ordering::Equal => serializer.serialize_i8(0),
            Ordering::Greater => serializer.serialize_i8(1),
        }
    }
}
