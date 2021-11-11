use std::convert::TryFrom;
use std::fmt;

/// The identifier of a source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(transparent)]
pub struct SourceId {
    index: u32,
}

impl SourceId {
    /// The empty source identifier.
    pub const EMPTY: Self = Self::empty();

    /// Construct a source identifier from an index.
    pub const fn new(index: u32) -> Self {
        Self { index }
    }

    /// Define an empty source identifier that cannot reference a source.
    pub const fn empty() -> Self {
        Self { index: u32::MAX }
    }

    /// Access the source identifier as an index.
    pub fn into_index(self) -> usize {
        self.index as usize
    }
}

impl fmt::Display for SourceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.index.fmt(f)
    }
}

impl Default for SourceId {
    fn default() -> Self {
        Self::empty()
    }
}

impl TryFrom<usize> for SourceId {
    type Error = std::num::TryFromIntError;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        Ok(Self {
            index: u32::try_from(value)?,
        })
    }
}

impl serde::Serialize for SourceId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.index.serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for SourceId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Self {
            index: u32::deserialize(deserializer)?,
        })
    }
}
