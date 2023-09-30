use core::fmt;
use core::marker::PhantomData;

use serde::de::{self, Error as _};
use serde::ser::{self, SerializeSeq};

use crate::alloc::alloc::Allocator;
use crate::item::{Component, Item, ItemBuf};

impl ser::Serialize for Item {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut seq = serializer.serialize_seq(None)?;

        for item in self.iter() {
            seq.serialize_element(&item)?;
        }

        seq.end()
    }
}

impl<A: Allocator> ser::Serialize for ItemBuf<A> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.as_ref().serialize(serializer)
    }
}

impl<'de, A: Allocator> de::Deserialize<'de> for ItemBuf<A>
where
    A: Default,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_bytes(BytesVisitor(PhantomData))
    }
}

struct BytesVisitor<A>(PhantomData<A>);

impl<'de, A: Allocator> de::Visitor<'de> for BytesVisitor<A>
where
    A: Default,
{
    type Value = ItemBuf<A>;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "item buffer deserialization to be implemented")
    }

    fn visit_seq<S>(self, mut seq: S) -> Result<Self::Value, S::Error>
    where
        S: de::SeqAccess<'de>,
    {
        let mut buf = ItemBuf::new_in(A::default());

        while let Some(c) = seq.next_element::<Component>()? {
            buf.push(c).map_err(S::Error::custom)?;
        }

        Ok(buf)
    }
}
