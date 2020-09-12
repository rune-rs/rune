use crate::{Hash, StaticType};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops;

/// The type of an entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Type(Hash);

impl Type {
    /// Construct type from a Rust type identifier.
    pub fn from_type_id(type_id: std::any::TypeId) -> Self {
        Self(Hash::from_type_id(type_id))
    }

    /// Construct a type from a type hash.
    pub const fn from_type_hash(hash: Hash) -> Self {
        Self(hash)
    }
}

impl From<&'static StaticType> for Type {
    fn from(static_type: &'static StaticType) -> Self {
        Self(static_type.hash)
    }
}

impl From<Hash> for Type {
    fn from(hash: Hash) -> Self {
        Self(hash)
    }
}

impl ops::Deref for Type {
    type Target = Hash;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::Type;

    #[test]
    fn test_size() {
        assert_eq! {
            std::mem::size_of::<Type>(),
            8,
        };
    }
}
