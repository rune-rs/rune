use crate::{Hash, StaticType};
use std::cmp;
use std::fmt;
use std::hash;

/// The type of an entry.
#[derive(Debug, Clone, Copy)]
pub enum Type {
    /// The static type of a value.
    StaticType(&'static StaticType),
    /// The type hash of a type.
    Hash(Hash),
}

impl Type {
    /// Treat the value type as a type hash.
    pub fn as_type_hash(&self) -> Hash {
        match self {
            Self::StaticType(ty) => ty.hash,
            Self::Hash(hash) => *hash,
        }
    }
}

impl cmp::PartialEq for Type {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::StaticType(a), b) => match b {
                Self::StaticType(b) => a.eq(b),
                Self::Hash(b) => &a.hash == b,
            },
            (Self::Hash(a), b) => match b {
                Self::StaticType(b) => a == &b.hash,
                Self::Hash(b) => a == b,
            },
        }
    }
}

impl cmp::PartialEq<Hash> for Type {
    fn eq(&self, other: &Hash) -> bool {
        match self {
            Self::StaticType(a) => &a.hash == other,
            Self::Hash(hash) => hash == other,
        }
    }
}

impl cmp::Eq for Type {}

impl hash::Hash for Type {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        match self {
            Self::StaticType(ty) => ty.hash.hash(state),
            Self::Hash(hash) => hash.hash(state),
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StaticType(ty) => write!(f, "type({})", ty.name),
            Self::Hash(hash) => write!(f, "type({})", hash),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Type;

    #[test]
    fn test_size() {
        assert_eq! {
            std::mem::size_of::<Type>(),
            16,
        };
    }
}
